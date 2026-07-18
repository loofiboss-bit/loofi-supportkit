# Installation

## Fedora KDE 44 x64

The COPR package is the recommended Fedora installation because it participates in normal system updates.

```bash
sudo dnf copr enable loofitheboss/loofi-supportkit
sudo dnf install loofi-supportkit
```

Launch **Loofi SupportKit** from the application menu. To remove it later:

```bash
sudo dnf remove loofi-supportkit
sudo dnf copr disable loofitheboss/loofi-supportkit
```

The GitHub release also provides an RPM and an AppImage. Make the AppImage executable before launching it:

```bash
chmod +x Loofi-SupportKit-0.1.0-x86_64.AppImage
./Loofi-SupportKit-0.1.0-x86_64.AppImage
```

## Windows 11 x64

Download either the setup executable or portable executable from the [latest GitHub release](https://github.com/loofiboss-bit/loofi-supportkit/releases/latest).

The initial Windows artifacts are unsigned. Windows SmartScreen may therefore ask for confirmation. Verify the file against `SHA256SUMS.txt`, then use **More info → Run anyway** only if the checksum matches the release.

PowerShell can verify a download with:

```powershell
Get-FileHash .\Loofi-SupportKit-0.1.0-windows-x64-setup.exe -Algorithm SHA256
```

The portable executable stores cases in the same per-user application data location as the installed version. Neither build requires administrator access for diagnostics.

## Verify a release file

Every GitHub release includes `SHA256SUMS.txt`. On Fedora:

```bash
sha256sum --check SHA256SUMS.txt --ignore-missing
```

Only use files whose checksum reports `OK`. Release binaries are currently unsigned; checksums protect transfer integrity but are not a substitute for code signing.
