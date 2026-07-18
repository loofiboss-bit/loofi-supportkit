# Install Loofi SupportKit

## Fedora KDE 44 x64

Install from COPR so the application receives normal package updates:

```bash
sudo dnf copr enable loofitheboss/loofi-supportkit
sudo dnf install loofi-supportkit
```

Launch **Loofi SupportKit** from the application menu.

The [latest GitHub release](https://github.com/loofiboss-bit/loofi-supportkit/releases/latest) also contains an RPM and AppImage.

## Windows 11 x64

Download the setup executable or portable executable from the latest GitHub release. The initial Windows files are unsigned, so verify the SHA-256 digest against `SHA256SUMS.txt` before accepting a SmartScreen prompt.

```powershell
Get-FileHash .\Loofi-SupportKit-0.1.0-windows-x64-setup.exe -Algorithm SHA256
```

Continue with [your first diagnostic](First-Diagnostic).
