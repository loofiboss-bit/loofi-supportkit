# User guide

## Run a diagnostic

1. Open Loofi SupportKit and select **New diagnostic**.
2. Choose the category closest to the problem. Use **General diagnostic scan** when the cause is unclear.
3. Keep Wi-Fi-name redaction enabled unless the network name is essential to the support request.
4. Select **Run read-only scan**. Unavailable collectors are recorded and skipped; they do not make the whole case fail.

The scan reads system state only. It does not install packages, restart services, change settings, upload evidence, or request administrator access.

## Read the result

Each finding states its severity, confidence, supporting evidence, likely impact, and safe next steps. A partial scan means at least one collector was unavailable, failed, or timed out. Treat missing evidence as unknown, not healthy.

## Review privacy changes

Open **Privacy review** before exporting. It shows each redaction category, transformed JSON path, review warning, and the exact redacted evidence that will leave the application. Raw collection data is not saved to the case store.

## Export and share

- **ZIP** is the preferred support bundle. It includes a static HTML summary, structured JSON, evidence files, manifests, and SHA-256 checksums.
- **HTML** is an offline, script-free report for reading in a browser.
- **JSON** is the complete versioned case for tools and long-term storage.

Exports are written locally and are never uploaded automatically. Share the resulting file using a channel you trust.

## Reopen or remove a case

Use **Case history** to reopen a redacted local case. **Move to recovery** removes it from active history and moves the case document into the app recovery directory; existing exports are unchanged. The exact data and export paths are shown under **Settings**.

## Current limits

- Cancellation is represented in the case format, but active process cancellation is not yet exposed. Every collector has a fixed timeout.
- Windows live collectors and installers require Windows 11 x64 validation.
- Loofi SupportKit does not provide repair actions or claim Flatpak support.
