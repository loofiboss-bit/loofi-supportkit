# Loofi SupportKit

![Loofi SupportKit — Local. Read-only. Private.](https://raw.githubusercontent.com/loofiboss-bit/loofi-supportkit/main/docs/assets/github-banner.png)

Loofi SupportKit turns bounded, read-only system evidence into a privacy-reviewed support case. Diagnostics, rules, redaction, case storage, and export run locally. The application does not repair the host or upload evidence.

![Loofi SupportKit home screen](https://raw.githubusercontent.com/loofiboss-bit/loofi-supportkit/main/docs/screenshots/home.png)

## Start here

- [Install on Fedora or Windows](Install)
- [Run your first diagnostic](First-Diagnostic)
- [Review privacy and export a case](Privacy-and-Exports)
- [Troubleshoot the application](Troubleshooting)

## Supported systems

- Fedora KDE 44 x64
- Windows 11 x64

Other operating systems are reported as unsupported. Flatpak, repairs, remote access, accounts, telemetry, cloud upload, and AI diagnosis are outside the current product scope.

## Safety model

Collectors use a reviewed executable allowlist, fixed argument arrays, timeouts, and output limits. No collector invokes a shell. Raw evidence remains in memory during the scan; only the redacted case crosses the persistence and export boundary.
