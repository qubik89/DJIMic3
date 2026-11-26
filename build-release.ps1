#Requires -Version 5
#
# Build DJI Mic Control for Windows release with all path-privacy safeties on.
#
# Produces two artifacts in Release\windows\ :
#     DJI Mic Control.exe                     - portable, self-contained binary
#     DJI Mic Control_<ver>_x64-setup.exe     - NSIS installer
# The installer registers the app in Add/Remove Programs and the Start menu
# (so it's searchable) and auto-provisions the WebView2 runtime on install; the
# portable exe assumes WebView2 is already present. For macOS/Linux, use
# build-release.sh.
#
# "Safeties on" means every absolute filesystem path Rust would otherwise bake
# into panic/backtrace strings (which `strip` does NOT remove) is remapped away,
# so a crash can never leak the build machine's home directory, username, or
# hostname. The build then re-scans the compiled binary and FAILS if anything
# slipped through.
#
# The build runs in steps so CI can sign between them (docs/windows-signing.md):
#     -Step compile   build target\release\djimic-gui.exe
#     -Step bundle    package that exe, as it is, into the NSIS installer and
#                     collect both files in Release\windows
#     -Step verify    check that both files, and the exe inside the installer,
#                     carry a valid signature
# The default, -Step all, runs compile and bundle: an unsigned build.

param(
    [ValidateSet('all', 'compile', 'bundle', 'verify')]
    [string]$Step = 'all'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $ScriptDir

$Exe = Join-Path $ScriptDir 'target\release\djimic-gui.exe'
$Out = Join-Path $ScriptDir 'Release\windows'

# --- Path-privacy safety -----------------------------------------------------
# Strip the home dir and Cargo registry from embedded paths. CARGO_ENCODED_-
# RUSTFLAGS (0x1f-separated) tolerates spaces in paths. We emit both backslash
# and forward-slash prefix variants because rustc's embedded paths can use
# either separator on Windows; the post-build scan is the backstop.
Remove-Item Env:RUSTFLAGS -ErrorAction SilentlyContinue  # avoid cargo conflict
$UserHome  = $env:USERPROFILE
$CargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $UserHome '.cargo' }
$UserHomeF  = $UserHome  -replace '\\', '/'
$CargoHomeF = $CargoHome -replace '\\', '/'
$US = [string][char]0x1f
$remaps = @(
    "--remap-path-prefix=$CargoHome\=",
    "--remap-path-prefix=$CargoHomeF/=",
    "--remap-path-prefix=$UserHome\=/",
    "--remap-path-prefix=$UserHomeF/=/"
)
$env:CARGO_ENCODED_RUSTFLAGS = ($remaps -join $US)

# --- Leak scan: no build-machine paths in a shipped file ---------------------
$needles = @("$UserHome\", "$UserHomeF/", "$CargoHome\", "$CargoHomeF/",
             $env:USERNAME, [System.Net.Dns]::GetHostName()) |
           Where-Object { $_ } | Select-Object -Unique
function Assert-NoLeak([string]$path) {
    $bytes = [System.IO.File]::ReadAllBytes($path)
    $text  = [System.Text.Encoding]::GetEncoding(28591).GetString($bytes)  # Latin1
    $hit   = $needles | Where-Object { $text.Contains($_) }
    if ($hit) {
        Write-Error ("SECURITY: build-machine paths leaked into " +
                     "$([System.IO.Path]::GetFileName($path)): " + ($hit -join ', '))
        exit 1
    }
}

if ($Step -ne 'verify') {
    foreach ($tool in 'cargo', 'npm') {
        if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
            throw "$tool not found in PATH."
        }
    }
}

# --- Compile (Tauri runs the frontend `npm run build` itself) ----------------
if ($Step -in 'all', 'compile') {
    Write-Host '==> Compiling the Windows release exe'
    Push-Location gui
    try {
        npm install --no-audit --no-fund
        if ($LASTEXITCODE -ne 0) { throw 'npm install failed' }
        npx tauri build --no-bundle
        if ($LASTEXITCODE -ne 0) { throw 'tauri build failed' }
    } finally {
        Pop-Location
    }
    if (-not (Test-Path $Exe)) { throw "Expected binary not found: $Exe" }

    Assert-NoLeak $Exe
    Write-Host '==> Leak scan clean: no home path, Cargo path, username, or hostname in the exe.'

    # tauri-build embeds the Windows resource block (app icon + version info) at
    # compile time, so a populated VersionInfo confirms the icon is in the exe.
    $vi = (Get-Item $Exe).VersionInfo
    if ($vi.ProductName) {
        Write-Host "==> Logo/resources embedded (ProductName: $($vi.ProductName))."
    } else {
        Write-Warning 'exe has no embedded version resource - the app icon may be missing.'
    }
}

# --- Bundle: the NSIS installer around the exe as it is ----------------------
# The installer registers the app in Add/Remove Programs and the Start menu, and
# auto-provisions the WebView2 runtime on install.
# --no-binary-patching: otherwise Tauri writes the package type into the exe
# before packaging it, which would break a signature added after compiling.
# Only Tauri's updater reads that value, and this app doesn't use it.
if ($Step -in 'all', 'bundle') {
    Write-Host '==> Packaging the NSIS installer'
    if (-not (Test-Path $Exe)) { throw "$Exe not found: run -Step compile first." }
    Push-Location gui
    try {
        npx tauri bundle --bundles nsis --no-binary-patching
        if ($LASTEXITCODE -ne 0) { throw 'tauri bundle failed' }
    } finally {
        Pop-Location
    }

    if (Test-Path $Out) { Remove-Item -Recurse -Force $Out }
    New-Item -ItemType Directory -Force -Path $Out | Out-Null

    Copy-Item $Exe (Join-Path $Out 'DJI Mic Control.exe')
    Write-Host '    + DJI Mic Control.exe (portable)'

    $Setup = Get-ChildItem (Join-Path $ScriptDir 'target\release\bundle\nsis') `
                 -Filter '*-setup.exe' -ErrorAction SilentlyContinue |
             Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $Setup) { throw 'NSIS installer not found in target\release\bundle\nsis' }
    Copy-Item $Setup.FullName (Join-Path $Out $Setup.Name)
    Write-Host "    + $($Setup.Name) (installer)"

    Assert-NoLeak $Setup.FullName
    Write-Host '==> Leak scan clean: no home path, Cargo path, username, or hostname in the installer.'

    Write-Host '==> Done. Artifacts in Release\windows:'
    Get-ChildItem $Out | Select-Object -ExpandProperty Name
}

# --- Verify signatures (CI, after signing) -----------------------------------
if ($Step -eq 'verify') {
    Write-Host '==> Checking signatures'
    function Assert-Signed([string]$path) {
        $name = [System.IO.Path]::GetFileName($path)
        $sig  = Get-AuthenticodeSignature -LiteralPath $path
        if ($sig.Status -ne 'Valid') {
            throw "$name is not validly signed: $($sig.Status). $($sig.StatusMessage)"
        }
        Write-Host "    signed: $name ($($sig.SignerCertificate.Subject))"
    }

    $Setup = Get-ChildItem $Out -Filter '*-setup.exe' | Select-Object -First 1
    if (-not $Setup) { throw "No installer in $Out" }
    Assert-Signed (Join-Path $Out 'DJI Mic Control.exe')
    Assert-Signed $Setup.FullName

    # The installer must carry the signed exe, not an unsigned copy. 7-Zip can
    # unpack NSIS installers.
    $sevenZip = Get-Command 7z -ErrorAction SilentlyContinue
    $sevenZip = if ($sevenZip) { $sevenZip.Source } else { Join-Path $env:ProgramFiles '7-Zip\7z.exe' }
    if (-not (Test-Path $sevenZip)) { throw '7-Zip is needed to look inside the installer.' }
    $tmp = Join-Path ([System.IO.Path]::GetTempPath()) ([guid]::NewGuid().ToString())
    & $sevenZip x -y "-o$tmp" $Setup.FullName | Out-Null
    if ($LASTEXITCODE -ne 0) { throw '7-Zip could not unpack the installer.' }
    try {
        $inner = @(Get-ChildItem $tmp -Recurse -Filter 'djimic-gui.exe')
        if ($inner.Count -eq 0) { throw 'djimic-gui.exe not found inside the installer.' }
        foreach ($f in $inner) { Assert-Signed $f.FullName }
    } finally {
        Remove-Item -Recurse -Force $tmp
    }
    Write-Host '==> All signatures valid.'
}
