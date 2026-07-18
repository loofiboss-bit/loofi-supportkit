# Troubleshooting Loofi SupportKit

## A collector was skipped

Open the scan summary. The required platform command may be unavailable, or the host may be outside the supported Fedora KDE 44 x64 and Windows 11 x64 targets. Skipped evidence is treated as unknown.

## The scan is partial

Other collectors still complete. Review the missing collectors before sharing the case and tell the recipient what was unavailable.

## I cannot find an export

Open **Settings** to see the current application data and export directories. Case history stores only redacted case documents.

![Case history](https://raw.githubusercontent.com/loofiboss-bit/loofi-supportkit/main/docs/screenshots/case-history.png)

## Windows blocks the download

The initial Windows artifacts are unsigned. Verify the SHA-256 hash against the release checksum file before choosing **More info → Run anyway**.

## Fedora is not receiving updates

```bash
dnf copr list
sudo dnf upgrade --refresh
```

If the COPR is missing, return to the [installation guide](Install).

## Report a problem

Open a [GitHub issue](https://github.com/loofiboss-bit/loofi-supportkit/issues) with the application version, supported operating system, the visible error, and synthetic reproduction steps. Do not attach a real support bundle unless you have independently reviewed and removed personal data.
