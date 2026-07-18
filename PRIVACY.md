# Privacy

Loofi SupportKit processes diagnostics locally. It has no telemetry, account, cloud backend, or automatic upload. A scan may temporarily read system identifiers and bounded logs so that rules can evaluate evidence. The MVP persists and exports only the redacted case document.

Before export, the privacy review lists changed data categories and field paths. The redaction manifest stores counts, never original matched values. Because no heuristic can recognize every sensitive value, users must review free-text evidence and the unclassified-data warning before sharing.

Removing a case from history moves its app-owned case file to the recovery directory. Existing exports are separate files and are not changed automatically.
