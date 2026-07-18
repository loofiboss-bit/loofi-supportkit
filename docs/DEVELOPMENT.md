# Development

## Prerequisites

- Node.js 22.12 or newer
- Rust 1.85 or newer with `rustfmt` and `clippy`
- `just`
- Tauri 2 system dependencies for the host platform

On Fedora 44:

```bash
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
  libappindicator-gtk3-devel librsvg2-devel libxdo-devel glib2-devel \
  gcc gcc-c++ make
```

Install project dependencies and start the desktop app:

```bash
just setup
just dev
```

`npm run dev` starts the browser fixture adapter. It displays synthetic evidence and never probes the host.

## Repository gates

```bash
just check
npm run test:e2e
just package
```

`just check` covers Rust formatting, Clippy, Rust and React tests, TypeScript, ESLint, JSON Schema validation, and Prettier. Playwright requires a Chromium installation (`npx playwright install chromium`).

## Trusted-boundary rules

- Do not add shell execution.
- Renderer input must not select executables, arguments, or filesystem paths.
- New commands require a reviewed `Executable` variant and a bounded fixed argument vector.
- Set a timeout and output cap before collecting a new source.
- Treat collector failure as structured evidence, never as a reason to fabricate a healthy result.
- Redact before persistence and report generation.
- Add deterministic fixtures and adversarial tests for behavior changes.

## Windows validation

The `windows-latest` CI job compiles and tests the Windows collector path. The packaging workflow builds NSIS plus the unsigned portable executable. Live collector validation still requires a Windows 11 x64 host.

## Release checklist

1. Run `just check` and `npm run test:e2e`.
2. Build packages on each target OS.
3. Verify the ZIP file list and checksums using a synthetic case.
4. Run a non-elevated live scan on Fedora KDE 44 and Windows 11 x64.
5. Confirm case deletion/recovery and local export paths.
6. Update `CHANGELOG.md` and the version in Cargo, npm, and Tauri configuration.
