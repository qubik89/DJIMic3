#!/usr/bin/env bash
#
# Build DJI Mic Control for release with all path-privacy safeties enabled.
#
# Builds the shippable bundles for the host OS and copies only those into
# Release/<os>/ :
#     macOS  ->  Release/macos    (.app, .dmg)
#     Linux  ->  Release/linux    (.deb, .rpm, .AppImage)
# For Windows, use build-release.ps1.
#
# "Safeties on" means every absolute filesystem path Rust would otherwise bake
# into panic/backtrace strings (which `strip` does NOT remove) is remapped away,
# so a crash can never leak the build machine's home directory, username, or
# hostname. The build then re-scans the compiled binary and FAILS if anything
# slipped through.
#
# On macOS the build can also be signed with a Developer ID certificate and
# notarized by Apple (see "macOS signing & notarization" below and
# docs/macos-signing.md); without those credentials it stays unsigned.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

die() { echo "ERROR: $*" >&2; exit 1; }

# --- Homebrew cask -----------------------------------------------------------
# The cask in qubik89/homebrew-tap (Casks/mic-control.rb) downloads the
# release's .dmg and checks it against a sha256, so each release needs a new
# version and checksum there. update_cask writes both; it doesn't commit or
# push. Two ways to run it:
#
#   ./build-release.sh --update-cask <dmg> <homebrew-tap checkout>
#       Only updates the cask (no build). The release workflow does this after
#       publishing, with the .dmg it uploaded.
#   HOMEBREW_TAP_DIR=<homebrew-tap checkout> ./build-release.sh
#       Updates the cask after a macOS build. Only useful if that .dmg is the
#       one you upload: a rebuilt .dmg has a different checksum.
app_version() {
  sed -nE 's/^  "version": "([^"]+)",?$/\1/p' "$SCRIPT_DIR/gui/src-tauri/tauri.conf.json" | head -1
}
update_cask() { # <dmg> <tap dir>
  local dmg="$1" cask="$2/Casks/mic-control.rb" version sha tmp
  [ -f "$dmg" ]  || die "no such .dmg: $dmg"
  [ -f "$cask" ] || die "no cask at $cask"
  version="$(app_version)"
  [ -n "$version" ] || die "could not read the version from gui/src-tauri/tauri.conf.json"
  if command -v shasum >/dev/null 2>&1; then
    sha="$(shasum -a 256 "$dmg" | cut -d' ' -f1)"
  else
    sha="$(sha256sum "$dmg" | cut -d' ' -f1)"
  fi
  tmp="$(mktemp)"
  sed -E -e "s/^(  version \")[^\"]*(\")$/\1${version}\2/" \
         -e "s/^(  sha256 \")[0-9a-f]{64}(\")$/\1${sha}\2/" "$cask" >"$tmp"
  if ! grep -qxF "  version \"$version\"" "$tmp" || ! grep -qxF "  sha256 \"$sha\"" "$tmp"; then
    rm -f "$tmp"
    die "could not find the version and sha256 lines in $cask"
  fi
  cat "$tmp" >"$cask"
  rm -f "$tmp"
  echo "==> Homebrew cask: mic-control $version, sha256 $sha"
}

if [ "${1:-}" = "--update-cask" ]; then
  [ "$#" -eq 3 ] || die "usage: $0 --update-cask <dmg> <homebrew-tap checkout>"
  update_cask "$2" "$3"
  exit 0
fi

cd "$SCRIPT_DIR"

# --- Path-privacy safety -----------------------------------------------------
# Strip the home dir and Cargo registry from embedded paths. We use
# CARGO_ENCODED_RUSTFLAGS (0x1f-separated) rather than RUSTFLAGS so that home
# paths containing spaces are handled correctly. The registry remap has an
# empty replacement; the home remap maps to "/". Covering CARGO_HOME as well
# handles CI hosts that relocate it outside $HOME.
unset RUSTFLAGS 2>/dev/null || true   # avoid "both RUSTFLAGS set" cargo error
CARGO_HOME_DIR="${CARGO_HOME:-$HOME/.cargo}"
US=$'\x1f'
export CARGO_ENCODED_RUSTFLAGS="--remap-path-prefix=${CARGO_HOME_DIR}/=${US}--remap-path-prefix=${HOME}/=/"

# --- Host platform -----------------------------------------------------------
OS="$(uname -s)"
case "$OS" in
  Darwin) PLATFORM="macos"; BUNDLES="app,dmg" ;;
  Linux)  PLATFORM="linux"; BUNDLES="deb,rpm,appimage"
          # Let AppImage tooling run by extraction (no FUSE needed on CI hosts).
          export APPIMAGE_EXTRACT_AND_RUN=1
          # linuxdeploy's bundled `strip` (binutils 2.35) can't parse the
          # `.relr.dyn` section modern distro libs ship with, and aborts the
          # whole build. Our binary is already stripped by Cargo's
          # profile.release (strip = true) and the vendored system libs ship
          # pre-stripped, so this second strip pass is redundant anyway.
          export NO_STRIP=1 ;;
  *) echo "ERROR: unsupported OS '$OS' — use build-release.ps1 on Windows." >&2; exit 1 ;;
esac

command -v cargo >/dev/null 2>&1 || { echo "ERROR: cargo not found in PATH." >&2; exit 1; }
command -v npm   >/dev/null 2>&1 || { echo "ERROR: npm not found in PATH." >&2; exit 1; }

# --- macOS signing & notarization (optional) ---------------------------------
# Without credentials the build stays unsigned, as before. For a build that
# Gatekeeper opens on any Mac without warnings, set
#
#   APPLE_SIGNING_IDENTITY   "Developer ID Application: Name (TEAMID)", or that
#                            certificate's SHA-1 from
#                            `security find-identity -v -p codesigning`
#
# plus ONE set of notarization credentials:
#
#   APPLE_API_KEY, APPLE_API_ISSUER, APPLE_API_KEY_PATH
#       App Store Connect API key: key ID, issuer ID, path to AuthKey_*.p8
#   APPLE_ID, APPLE_PASSWORD, APPLE_TEAM_ID
#       Apple ID with an app-specific password
#
# Where the certificate isn't in the login keychain (CI), also set
# APPLE_CERTIFICATE (base64 .p12) and APPLE_CERTIFICATE_PASSWORD; Tauri imports
# it into a temporary keychain.
#
# Tauri signs with the hardened runtime and notarizes + staples the .app; this
# script then notarizes + staples the .dmg and checks both with Gatekeeper.
# NOTARIZE=0 signs without notarizing. That is for local testing only:
# Gatekeeper still blocks such a build on other Macs.
SIGN=0
SIGN_NAME=""
NOTARY_ARGS=()
NOTARY_ENV="APPLE_API_KEY APPLE_API_ISSUER APPLE_API_KEY_PATH APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID"

if [ "$PLATFORM" = "macos" ]; then
  # CI passes secrets that aren't configured as empty strings: treat as unset.
  for v in APPLE_SIGNING_IDENTITY APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD $NOTARY_ENV; do
    [ -n "${!v:-}" ] || unset "$v"
  done

  if [ -z "${APPLE_SIGNING_IDENTITY:-}" ]; then
    # Nothing to notarize without a signature; keep Tauri from trying.
    for v in $NOTARY_ENV; do unset "$v"; done
    echo "==> macOS: unsigned build (set APPLE_SIGNING_IDENTITY to sign; see docs/macos-signing.md)"
  else
    SIGN=1

    if [ -n "${APPLE_CERTIFICATE:-}" ]; then
      SIGN_NAME="$APPLE_SIGNING_IDENTITY"   # Tauri imports it during the build
    else
      matches="$(security find-identity -v -p codesigning | grep -F -- "$APPLE_SIGNING_IDENTITY" || true)"
      count="$(printf '%s' "$matches" | grep -c . || true)"
      [ "$count" -ne 0 ] || die "no valid signing identity matching \"$APPLE_SIGNING_IDENTITY\" in the keychain.
       List them with: security find-identity -v -p codesigning"
      [ "$count" -eq 1 ] || die "\"$APPLE_SIGNING_IDENTITY\" matches $count identities; set APPLE_SIGNING_IDENTITY to the SHA-1 of the one to use:
$matches"
      SIGN_NAME="$(printf '%s\n' "$matches" | sed -E 's/^[^"]*"(.*)"[^"]*$/\1/')"
    fi

    require() { for v in "$@"; do [ -n "${!v:-}" ] || die "$v is not set (needed with $*)"; done; }
    if [ "${NOTARIZE:-1}" = "0" ]; then
      for v in $NOTARY_ENV; do unset "$v"; done
      echo "WARN: NOTARIZE=0: signed but NOT notarized; Gatekeeper will block this build on other Macs." >&2
    elif [ -n "${APPLE_API_KEY:-}${APPLE_API_ISSUER:-}${APPLE_API_KEY_PATH:-}" ]; then
      require APPLE_API_KEY APPLE_API_ISSUER APPLE_API_KEY_PATH
      [ -f "$APPLE_API_KEY_PATH" ] || die "APPLE_API_KEY_PATH: no such file: $APPLE_API_KEY_PATH"
      unset APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID   # one method at a time
      NOTARY_ARGS=(--key "$APPLE_API_KEY_PATH" --key-id "$APPLE_API_KEY" --issuer "$APPLE_API_ISSUER")
    elif [ -n "${APPLE_ID:-}${APPLE_PASSWORD:-}${APPLE_TEAM_ID:-}" ]; then
      require APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID
      NOTARY_ARGS=(--apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" --team-id "$APPLE_TEAM_ID")
    else
      die "APPLE_SIGNING_IDENTITY is set but there are no notarization credentials.
       Set APPLE_API_KEY + APPLE_API_ISSUER + APPLE_API_KEY_PATH (or APPLE_ID +
       APPLE_PASSWORD + APPLE_TEAM_ID), or NOTARIZE=0 for a local test build."
    fi

    if [ "${#NOTARY_ARGS[@]}" -gt 0 ]; then
      case "$SIGN_NAME" in
        "Developer ID Application:"*) ;;
        *) die "notarization needs a \"Developer ID Application\" certificate, not \"$SIGN_NAME\"." ;;
      esac
      echo "==> macOS: signing as \"$SIGN_NAME\" and notarizing"
    else
      echo "==> macOS: signing as \"$SIGN_NAME\" (no notarization)"
    fi
  fi
fi

OUT="$SCRIPT_DIR/Release/$PLATFORM"
BUNDLE_DIR="$SCRIPT_DIR/target/release/bundle"

echo "==> Building $PLATFORM release (bundles: $BUNDLES)"

# --- Build (Tauri runs the frontend `npm run build` itself) ------------------
(
  cd gui
  npm install --no-audit --no-fund
  npx tauri build --bundles "$BUNDLES"
)

# --- Collect artifacts -------------------------------------------------------
rm -rf "$OUT"
mkdir -p "$OUT"

copy_glob() { # <dir> <glob>
  local dir="$1" pat="$2" found=0 f
  shopt -s nullglob
  for f in "$dir"/$pat; do
    cp -R "$f" "$OUT/"
    echo "    + $(basename "$f")"
    found=1
  done
  shopt -u nullglob
  if [ "$found" -ne 1 ]; then
    echo "ERROR: no artifact matching '$pat' in $dir" >&2
    exit 1
  fi
}

if [ "$PLATFORM" = "macos" ]; then
  copy_glob "$BUNDLE_DIR/macos" "*.app"
  copy_glob "$BUNDLE_DIR/dmg"   "*.dmg"
  APP="$(ls -d "$OUT"/*.app | head -1)"
  DMG="$(ls "$OUT"/*.dmg | head -1)"
else
  copy_glob "$BUNDLE_DIR/deb"      "*.deb"
  copy_glob "$BUNDLE_DIR/rpm"      "*.rpm"
  copy_glob "$BUNDLE_DIR/appimage" "*.AppImage"
fi

# --- macOS: sign, notarize and staple the .dmg -------------------------------
# Tauri has already signed (and, with credentials, notarized + stapled) the
# .app inside it. Notarizing the .dmg as well lets Gatekeeper clear the
# download itself, even offline.
if [ "$SIGN" = 1 ]; then
  if ! codesign --verify "$DMG" 2>/dev/null; then
    echo "==> Signing $(basename "$DMG")"
    codesign --sign "$APPLE_SIGNING_IDENTITY" --timestamp "$DMG"
  fi
  if [ "${#NOTARY_ARGS[@]}" -gt 0 ]; then
    echo "==> Notarizing $(basename "$DMG") (usually takes a few minutes)"
    NOTARY_OUT="$(mktemp)"
    xcrun notarytool submit "$DMG" "${NOTARY_ARGS[@]}" --wait --timeout 1h \
      --output-format json >"$NOTARY_OUT" || true
    NOTARY_STATUS="$(plutil -extract status raw -o - "$NOTARY_OUT" 2>/dev/null || echo unknown)"
    if [ "$NOTARY_STATUS" != "Accepted" ]; then
      cat "$NOTARY_OUT" >&2
      NOTARY_ID="$(plutil -extract id raw -o - "$NOTARY_OUT" 2>/dev/null || true)"
      if [ -n "$NOTARY_ID" ]; then
        xcrun notarytool log "$NOTARY_ID" "${NOTARY_ARGS[@]}" >&2 || true
      fi
      rm -f "$NOTARY_OUT"
      die "Apple did not accept the .dmg for notarization (status: $NOTARY_STATUS)."
    fi
    rm -f "$NOTARY_OUT"
    xcrun stapler staple "$DMG"
  fi
fi

# --- Verify no build-machine paths leaked into any shipped file --------------
# Scans our own binary plus (on Linux) every file linuxdeploy bundled into the
# AppImage's AppDir, since NO_STRIP=1 above means those files keep whatever
# they shipped with rather than passing through a second strip.
BIN="$SCRIPT_DIR/target/release/djimic-gui"
SCAN_FILES=()
[ -f "$BIN" ] && SCAN_FILES+=("$BIN")
if [ "$PLATFORM" = "linux" ]; then
  APPDIR="$(ls -d "$BUNDLE_DIR"/appimage/*.AppDir 2>/dev/null | head -1)"
  if [ -n "$APPDIR" ]; then
    while IFS= read -r -d '' f; do SCAN_FILES+=("$f"); done < <(find "$APPDIR" -type f -print0)
  fi
fi

if [ "${#SCAN_FILES[@]}" -gt 0 ] && command -v strings >/dev/null 2>&1; then
  HOST="$(hostname -s 2>/dev/null || hostname 2>/dev/null || true)"
  USER_NAME="$(id -un 2>/dev/null || whoami 2>/dev/null || true)"
  # On CI the account is a generic word ("runner") that shows up by chance in
  # bundled system libraries; the home-path checks above still apply there.
  if [ -n "${GITHUB_ACTIONS:-}" ]; then HOST=""; USER_NAME=""; fi
  LEAK_REPORT=""
  for f in "${SCAN_FILES[@]}"; do
    # Grep only the file's own string content (not `strings -f`'s filename
    # prefix) — the AppDir path itself lives under $HOME and would otherwise
    # false-positive against the very patterns we're checking for.
    m="$(strings -a "$f" 2>/dev/null \
          | grep -aF -e "$HOME/" -e "$CARGO_HOME_DIR/" \
                 ${HOST:+-e "$HOST"} ${USER_NAME:+-e "$USER_NAME"} \
          | sort -u || true)"
    [ -n "$m" ] && LEAK_REPORT+=$'\n'"-- $f --"$'\n'"$m"$'\n'
  done
  if [ -n "$LEAK_REPORT" ]; then
    echo "SECURITY: build-machine paths leaked into a shipped file:" >&2
    printf '%s\n' "$LEAK_REPORT" | head -50 >&2
    exit 1
  fi
  echo "==> Leak scan clean: no home path, Cargo path, username, or hostname in ${#SCAN_FILES[@]} scanned file(s)."
else
  echo "WARN: skipped leak scan ('strings' unavailable or binary missing)." >&2
fi

# --- Verify logo is embedded, and (Linux) that the udev rule + reload script --
# --- ship in BOTH the .deb and .rpm, so the two never drift apart ------------
UDEV_RULE="60-dji-mic.rules"

if [ "$PLATFORM" = "macos" ]; then
  if [ -n "$APP" ] && [ -f "$APP/Contents/Resources/icon.icns" ]; then
    echo "==> Logo present: $(basename "$APP")/Contents/Resources/icon.icns"
  else
    echo "ERROR: app logo (Contents/Resources/icon.icns) missing from the .app" >&2
    exit 1
  fi

  # --- Verify the signature, hardened runtime and notarization ---------------
  if [ "$SIGN" = 1 ]; then
    codesign --verify --deep --strict "$APP" || die "the .app signature does not verify."
    SIG_INFO="$(codesign -dv --verbose=4 "$APP" 2>&1)"
    printf '%s\n' "$SIG_INFO" | grep -q '^CodeDirectory .*flags=.*runtime' \
      || die "the .app is signed without the hardened runtime."
    case "$SIGN_NAME" in
      *" "*) printf '%s\n' "$SIG_INFO" | grep -qxF "Authority=$SIGN_NAME" \
               || die "the .app is not signed by \"$SIGN_NAME\"." ;;
    esac
    codesign --verify "$DMG" || die "the .dmg signature does not verify."
    echo "==> Signed with the hardened runtime: $(basename "$APP"), $(basename "$DMG")"

    if [ "${#NOTARY_ARGS[@]}" -gt 0 ]; then
      xcrun stapler validate "$APP" >/dev/null || die "no notarization ticket stapled to the .app."
      xcrun stapler validate "$DMG" >/dev/null || die "no notarization ticket stapled to the .dmg."
      spctl --assess --type execute -vv "$APP" 2>&1 | grep -q "source=Notarized Developer ID" \
        || die "Gatekeeper does not accept the .app as notarized."
      spctl --assess --type open --context context:primary-signature -vv "$DMG" 2>&1 \
        | grep -q "source=Notarized Developer ID" \
        || die "Gatekeeper does not accept the .dmg as notarized."
      echo "==> Notarized and stapled; Gatekeeper accepts both."
    fi
  fi
else
  # List a package's payload paths, using whatever tooling the host has.
  deb_contents() {
    if command -v dpkg-deb >/dev/null 2>&1; then
      dpkg-deb -c "$1" | awk '{print $NF}'
    elif command -v ar >/dev/null 2>&1; then
      ar p "$1" "$(ar t "$1" | grep -m1 '^data\.tar')" | tar tf - 2>/dev/null
    fi
  }
  rpm_contents() {
    if command -v rpm >/dev/null 2>&1; then
      rpm -qlp "$1" 2>/dev/null
    elif command -v rpm2cpio >/dev/null 2>&1 && command -v cpio >/dev/null 2>&1; then
      rpm2cpio "$1" | cpio -t 2>/dev/null
    fi
  }

  DEB="$(ls "$OUT"/*.deb 2>/dev/null | head -1)"
  RPM="$(ls "$OUT"/*.rpm 2>/dev/null | head -1)"
  DEB_LIST="$(deb_contents "$DEB")"
  RPM_LIST="$(rpm_contents "$RPM")"

  assert_in() { # <label> <listing> <needle> <what>
    local label="$1" listing="$2" needle="$3" what="$4"
    if [ -z "$listing" ]; then
      echo "WARN: cannot inspect $label (no tool); skipped $what check." >&2
      return 0
    fi
    if printf '%s\n' "$listing" | grep -q "$needle"; then
      echo "==> $label includes $what"
    else
      echo "ERROR: $label is missing $what ($needle)" >&2
      exit 1
    fi
  }

  # udev rule parity — the whole point: present in BOTH packages.
  assert_in ".deb" "$DEB_LIST" "$UDEV_RULE"      "the udev rule"
  assert_in ".rpm" "$RPM_LIST" "$UDEV_RULE"      "the udev rule"
  # Same logo shipped by each Linux bundle (hicolor PNG).
  assert_in ".deb" "$DEB_LIST" "hicolor/128x128" "the app logo"
  assert_in ".rpm" "$RPM_LIST" "hicolor/128x128" "the app logo"

  # The udev-reload maintainer script must ship in the .deb too (rpm scriptlets
  # are embedded in the header and always travel with it).
  if command -v dpkg-deb >/dev/null 2>&1 && [ -n "$DEB" ]; then
    if dpkg-deb --ctrl-tarfile "$DEB" 2>/dev/null | tar t 2>/dev/null | grep -q 'postinst'; then
      echo "==> .deb ships a postinst (udev-reload) script"
    else
      echo "ERROR: .deb has no postinst script — the udev reload would not run" >&2
      exit 1
    fi
  fi
fi

if [ "$PLATFORM" = "macos" ] && [ -n "${HOMEBREW_TAP_DIR:-}" ]; then
  update_cask "$DMG" "$HOMEBREW_TAP_DIR"
fi

echo "==> Done. Artifacts in Release/$PLATFORM:"
ls -1 "$OUT"
