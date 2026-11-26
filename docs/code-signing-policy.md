# Code signing policy

Free code signing provided by [SignPath.io](https://about.signpath.io/),
certificate by [SignPath Foundation](https://signpath.org/).

## What is signed

The Windows release files of DJI Mic Control:

- the app, `DJI-Mic-Control_Windows_x64_portable.exe`, which is also the
  program the installer puts on your computer;
- the installer, `DJI-Mic-Control_Windows_x64_setup.exe`.

Only files built from this repository's source code are signed. They are
built by GitHub Actions ([release.yml](../.github/workflows/release.yml)) from
a version tag, never on a personal computer, and every release is approved by
hand before it is signed. The signature names SignPath Foundation as the
publisher. The steps are described in
[windows-signing.md](windows-signing.md).

## Team roles

- Committers and reviewers: [qubik89](https://github.com/qubik89)
- Approvers: [qubik89](https://github.com/qubik89)

## Privacy policy

This program will not transfer any information to other networked systems
unless specifically requested by the user or the person installing or
operating it.

The app talks only to the DJI receiver plugged in over USB. If Microsoft's
WebView2 runtime is missing, the installer downloads it from Microsoft.
