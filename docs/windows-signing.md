# Signing the Windows build

Windows SmartScreen warns about an unsigned `.exe` downloaded from the
internet ("Windows protected your PC"). The release workflow signs the Windows
files through [SignPath](https://about.signpath.io/), which gives open-source
projects a free certificate issued to SignPath Foundation. Without the
SignPath settings below, `release.yml` keeps producing unsigned builds.

A signature doesn't silence SmartScreen on day one: the warning goes away once
the certificate has built up reputation with enough downloads. Unlike an
unsigned file, whose reputation starts from zero with every release, that
reputation carries over to the next versions.

## How a release is signed

On the Windows runner, `release.yml`:

1. compiles the app (`build-release.ps1 -Step compile`);
2. uploads `djimic-gui.exe` and asks SignPath to sign it;
3. packages that signed exe into the NSIS installer
   (`build-release.ps1 -Step bundle`), so the installed app is signed too;
4. uploads the installer and asks SignPath to sign it;
5. checks that the portable exe, the installer and the exe inside the
   installer all carry a valid signature (`build-release.ps1 -Step verify`).

That is two signing requests per release, and SignPath Foundation requires a
person to approve each one. SignPath emails the approvers; the job waits up to
an hour for each approval and fails after that. Run the workflow again for the
same tag from the Actions tab if it times out.

The uninstaller that the installer writes on install is not signed.

## One-time setup

### 1. Requirements

SignPath Foundation's [conditions](https://signpath.org/terms), in short: an
OSI-approved license (the Unlicense is one), no proprietary code, an actively
maintained project with a release already out, multi-factor authentication for
every team member on GitHub and SignPath, and a code signing policy linked from
the home page and the download page. The policy is
[code-signing-policy.md](code-signing-policy.md), linked from the README and
from each release's notes. The website also needs a link to it.

### 2. Apply

Apply at [signpath.org/apply](https://signpath.org/apply) with the repository
URL. Once accepted, SignPath sets up an organization and a project for it.

### 3. Configure the SignPath project

1. Link the project to the predefined **GitHub.com** trusted build system.
2. Install the [SignPath GitHub App](https://github.com/apps/signpath) on the
   repository; SignPath uses it to check where each build came from.
3. Set the project's default artifact configuration to the contents of
   [signpath-artifact-configuration.xml](signpath-artifact-configuration.xml).
   It signs the single `.exe` in each request and requires product name
   `DJI Mic Control` and the version from `gui/src-tauri/tauri.conf.json`.
4. Create an API token for a user with submitter permission on the
   `release-signing` signing policy.

### 4. Configure GitHub

| Kind | Name | Value |
| --- | --- | --- |
| Secret | `SIGNPATH_API_TOKEN` | the API token |
| Variable | `SIGNPATH_ORGANIZATION_ID` | the organization ID (setting it turns signing on) |
| Variable | `SIGNPATH_PROJECT_SLUG` | the project slug |
| Variable | `SIGNPATH_SIGNING_POLICY_SLUG` | optional, defaults to `release-signing` |

With the GitHub CLI:

```bash
gh secret set SIGNPATH_API_TOKEN
gh variable set SIGNPATH_ORGANIZATION_ID --body "organization-id"
gh variable set SIGNPATH_PROJECT_SLUG --body "project-slug"
```

Don't publish a release with the `test-signing` policy: its certificate isn't
trusted by Windows, so the verify step fails.

## After the first signed release

- Check the first signing request in SignPath. If it rejects the product
  version, compare the version SignPath reports with `tauri.conf.json` and
  adjust `product-version` in the artifact configuration.
- On Windows, the installer's Properties → Digital Signatures tab should show
  SignPath Foundation.
- Update the README's and the website's note about the SmartScreen warning.
