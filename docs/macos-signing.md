# Signing and notarizing the macOS build

An unsigned `.app` downloaded from the internet is blocked by Gatekeeper
("damaged" / "can't be opened"). Signed with a **Developer ID** certificate and
notarized by Apple, it opens normally on any Mac. `build-release.sh` does both
when it finds the credentials below; without them it builds unsigned, as before.

This is distribution outside the Mac App Store (GitHub releases, a website).

## One-time setup

You need a paid Apple Developer Program membership.

### 1. Developer ID Application certificate

Only the account holder can create it.

1. Xcode → Settings → Accounts → select the team → **Manage Certificates…**
   → **+** → **Developer ID Application**. Xcode puts the certificate and its
   private key in the login keychain.
2. Check it is there:
   ```bash
   security find-identity -v -p codesigning
   ```
   You should see `"Developer ID Application: Your Name (TEAMID)"`.
3. Back it up: in Keychain Access, export the certificate **with its private
   key** as a `.p12` with a password. You need it for CI and for any other Mac;
   Apple only lets you create a few Developer ID certificates.

### 2. Notarization credentials

Recommended: an **App Store Connect API key**.

1. App Store Connect → Users and Access → Integrations → App Store Connect API
   → Team Keys → **+**, access **Developer**.
2. Download `AuthKey_<KEYID>.p8`. Apple lets you download it only once. Keep it
   outside the repository, e.g. in `~/.appstoreconnect/private_keys/`.
3. Note the **Key ID** and the **Issuer ID** shown on that page.

Alternative: your Apple ID with an app-specific password (account.apple.com →
Sign-In and Security → App-Specific Passwords) and your Team ID. Use
`APPLE_ID`, `APPLE_PASSWORD` and `APPLE_TEAM_ID` instead of the three
`APPLE_API_*` variables below.

## Build a signed release locally

```bash
export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAMID)"
export APPLE_API_KEY="KEYID"
export APPLE_API_ISSUER="issuer-uuid"
export APPLE_API_KEY_PATH="$HOME/.appstoreconnect/private_keys/AuthKey_KEYID.p8"
./build-release.sh
```

The script:

1. checks the identity is in the keychain and is a Developer ID certificate;
2. lets Tauri sign the `.app` with the hardened runtime, notarize it and staple
   the ticket;
3. signs, notarizes and staples the `.dmg`;
4. verifies both signatures, the hardened runtime, the stapled tickets and the
   Gatekeeper verdict (`source=Notarized Developer ID`), and fails otherwise.

Notarization adds a few minutes and needs internet access. If Apple rejects a
submission, the script prints Apple's log.

If two certificates share the same name (e.g. after renewing one), set
`APPLE_SIGNING_IDENTITY` to the SHA-1 shown by `security find-identity`.

To try signing without notarizing, add `NOTARIZE=0`. Any signing identity works
then, including an Apple Development one, but the result only opens on Macs
that already trust that certificate.

## Signed releases from GitHub Actions

`release.yml` passes these repository secrets to the macOS build. When they are
missing it keeps producing unsigned builds.

| Secret | Value |
| --- | --- |
| `APPLE_CERTIFICATE` | the `.p12`, base64-encoded |
| `APPLE_CERTIFICATE_PASSWORD` | the `.p12` password |
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: Your Name (TEAMID)` |
| `APPLE_API_KEY` | Key ID |
| `APPLE_API_ISSUER` | Issuer ID |
| `APPLE_API_KEY_P8` | contents of `AuthKey_<KEYID>.p8` |

With the GitHub CLI, from the folder holding the files:

```bash
base64 -i DeveloperID.p12 | gh secret set APPLE_CERTIFICATE
gh secret set APPLE_CERTIFICATE_PASSWORD
gh secret set APPLE_SIGNING_IDENTITY --body "Developer ID Application: Your Name (TEAMID)"
gh secret set APPLE_API_KEY --body "KEYID"
gh secret set APPLE_API_ISSUER --body "issuer-uuid"
gh secret set APPLE_API_KEY_P8 < AuthKey_KEYID.p8
```

Once a signed release is out, the "clear the quarantine flag" step in the
README's macOS install instructions is no longer needed.
