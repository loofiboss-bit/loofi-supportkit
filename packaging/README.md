# Packaging

Tauri owns the platform package configuration in `apps/desktop/src-tauri/tauri.conf.json`.

## Fedora KDE 44 x64

The supported artifacts are RPM and AppImage. Build on Fedora 44 so the RPM and linked runtime match the supported baseline:

```bash
NO_STRIP=1 npm run tauri --workspace @loofi/supportkit-desktop -- build --bundles rpm,appimage
```

`NO_STRIP=1` avoids the older `strip` binary embedded in linuxdeploy, which cannot read Fedora 44 libraries containing ELF RELR sections. Rust release binaries are still optimized; the bundled system libraries are left intact.

The public COPR package is defined by `packaging/fedora/loofi-supportkit.spec`. COPR builds the locked Cargo and npm dependency graphs from the tagged GitHub source. The project intentionally enables network access during builds because the first release does not vendor those dependency graphs.

Submit a tagged release to COPR with:

```bash
copr-cli build loofi-supportkit packaging/fedora/loofi-supportkit.spec \
  -r fedora-44-x86_64
```

## Windows 11 x64

The supported installer is NSIS. The release executable beside the bundle output is the portable build and does not require installation:

```powershell
npm run tauri --workspace @loofi/supportkit-desktop -- build --bundles nsis
```

Artifacts are unsigned development builds until a release pipeline supplies platform signing credentials. No signing material belongs in the repository.

## Flatpak status

Flatpak is not claimed. Its sandbox permissions and collector behavior require a separate design and validation effort.
