use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Cursor, Write};
use std::path::Path;
use supportkit_core::{Finding, Severity, SupportCase};
use zip::write::SimpleFileOptions;

pub fn build_html(case: &SupportCase) -> String {
    let findings = case
        .findings
        .iter()
        .map(finding_html)
        .collect::<Vec<_>>()
        .join("\n");
    let incomplete = case
        .collection_manifest
        .iter()
        .filter(|record| !matches!(record.status, supportkit_core::CollectorState::Complete))
        .map(|record| {
            format!(
                "<li><code>{}</code>: {:?}</li>",
                escape(&record.collector_id),
                record.status
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Loofi SupportKit case {case_id}</title><style>
body{{font:16px/1.55 system-ui,sans-serif;color:#19231f;background:#f4f7f5;margin:0}}main{{max-width:920px;margin:auto;padding:48px 24px}}header,.card,section{{background:#fff;border:1px solid #dce5df;border-radius:14px;padding:24px;margin:0 0 18px}}h1,h2,h3{{line-height:1.2}}.meta{{color:#52645b}}.severity{{display:inline-block;padding:3px 9px;border-radius:999px;background:#e8efeb;font-weight:700}}code{{overflow-wrap:anywhere}}details{{margin-top:12px}}footer{{color:#52645b;font-size:.9rem}}</style></head><body><main>
<header><p>Loofi SupportKit</p><h1>Support case</h1><p class="meta">Case {case_id} · Collected {created_at} · {platform:?}</p><p>This report contains privacy-reviewed diagnostic evidence. It does not contain repairs or AI-generated diagnosis.</p></header>
<section><h2>Summary</h2><p>{finding_count} findings for category <strong>{category:?}</strong>. Scan status: <strong>{scan_status:?}</strong>.</p></section>
<section><h2>Findings</h2>{findings_empty}{findings}</section>
<section><h2>Technician details</h2><dl><dt>Tool version</dt><dd>{app_version}</dd><dt>Case schema</dt><dd>{schema_version}</dd><dt>Platform</dt><dd>{platform:?}</dd></dl><h3>Incomplete collectors</h3><ul>{incomplete_empty}{incomplete}</ul></section>
<section><h2>Privacy review</h2><p>{redaction_count} values or fields were transformed. Review <code>redaction-manifest.json</code> for category counts and field paths. Original values are not recorded in the manifest.</p></section>
<footer>Generated locally by Loofi SupportKit. This static report works offline and contains no report JavaScript.</footer>
</main></body></html>"#,
        case_id = case.case_id,
        created_at = case.created_at.to_rfc3339(),
        platform = case.platform,
        category = case.category,
        scan_status = case.scan_status,
        finding_count = case.findings.len(),
        findings_empty = if case.findings.is_empty() {
            "<p>No diagnostic findings.</p>"
        } else {
            ""
        },
        findings = findings,
        app_version = escape(&case.app_version),
        schema_version = escape(&case.schema_version),
        incomplete_empty = if incomplete.is_empty() {
            "<li>None</li>"
        } else {
            ""
        },
        incomplete = incomplete,
        redaction_count = case.redaction_manifest.categories.values().sum::<u64>(),
    )
}

fn finding_html(finding: &Finding) -> String {
    let steps = finding
        .recommended_next_steps
        .iter()
        .map(|step| format!("<li>{}</li>", escape(step)))
        .collect::<Vec<_>>()
        .join("");
    let evidence = finding
        .evidence_refs
        .iter()
        .map(|reference| format!("<li><code>{}</code></li>", escape(reference)))
        .collect::<Vec<_>>()
        .join("");
    format!(
        "<article class=\"card\"><span class=\"severity\">{}</span><h3>{}</h3><p>{}</p><p><strong>Likely impact:</strong> {}</p><h4>Recommended next steps</h4><ol>{}</ol><details><summary>Evidence references</summary><ul>{}</ul><p>Rule <code>{}</code> version {}</p></details></article>",
        severity_label(finding.severity),
        escape(&finding.title),
        escape(&finding.explanation),
        escape(&finding.likely_impact),
        steps,
        evidence,
        escape(&finding.rule_id),
        finding.rule_version,
    )
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Critical => "Critical",
        Severity::Warning => "Warning",
        Severity::Notice => "Notice",
        Severity::Info => "Information",
    }
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn payload_files(case: &SupportCase) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut files = BTreeMap::new();
    files.insert("summary.html".into(), build_html(case).into_bytes());
    files.insert("findings.json".into(), pretty(&case.findings)?);
    files.insert("system-profile.json".into(), pretty(&case.system_profile)?);
    files.insert(
        "collection-manifest.json".into(),
        pretty(&case.collection_manifest)?,
    );
    files.insert(
        "redaction-manifest.json".into(),
        pretty(&case.redaction_manifest)?,
    );
    for evidence in &case.evidence {
        let safe_id: String = evidence
            .id
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || ".-_".contains(character) {
                    character
                } else {
                    '_'
                }
            })
            .collect();
        files.insert(format!("evidence/{safe_id}.json"), pretty(evidence)?);
    }
    let mut checksums = String::new();
    for (name, contents) in &files {
        checksums.push_str(&format!("{:x}  {name}\n", Sha256::digest(contents)));
    }
    files.insert("checksums.sha256".into(), checksums.into_bytes());
    Ok(files)
}

fn pretty(value: &impl serde::Serialize) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub fn zip_bytes(case: &SupportCase) -> Result<Vec<u8>> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut cursor);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .last_modified_time(zip::DateTime::default())
            .unix_permissions(0o644);
        for (name, contents) in payload_files(case)? {
            writer.start_file(name, options)?;
            writer.write_all(&contents)?;
        }
        writer.finish()?;
    }
    Ok(cursor.into_inner())
}

pub fn write_zip(case: &SupportCase, destination: &Path) -> Result<()> {
    let bytes = zip_bytes(case)?;
    let mut file = File::create(destination).context("create ZIP destination")?;
    file.write_all(&bytes).context("write ZIP bundle")?;
    file.sync_all().context("flush ZIP bundle")
}

pub fn write_html(case: &SupportCase, destination: &Path) -> Result<()> {
    std::fs::write(destination, build_html(case)).context("write HTML report")
}

pub fn write_json(case: &SupportCase, destination: &Path) -> Result<()> {
    std::fs::write(destination, pretty(case)?).context("write JSON support case")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::collections::BTreeMap;
    use supportkit_core::*;
    use uuid::Uuid;

    fn case() -> SupportCase {
        SupportCase {
            schema_version: CASE_SCHEMA_VERSION.into(),
            case_id: Uuid::nil(),
            created_at: chrono::Utc.with_ymd_and_hms(2026, 7, 18, 10, 0, 0).unwrap(),
            app_version: "0.1.0".into(),
            platform: Platform::FedoraKde44X64,
            category: ProblemCategory::General,
            scan_status: ScanStatus::Complete,
            system_profile: SystemProfile {
                os_name: "Fedora Linux".into(),
                os_version: "44".into(),
                architecture: "x86_64".into(),
                desktop: Some("KDE Plasma".into()),
                hostname: None,
            },
            collection_manifest: vec![],
            findings: vec![],
            evidence: vec![],
            redaction_manifest: RedactionManifest {
                version: 1,
                categories: BTreeMap::new(),
                transformed_paths: vec![],
                warnings: vec![],
            },
            export_history: vec![],
        }
    }

    #[test]
    fn report_is_offline_and_escapes_case_data() {
        let mut document = case();
        document.app_version = "<script>alert(1)</script>".into();
        let html = build_html(&document);
        assert!(html.starts_with("<!doctype html>"));
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn zip_is_deterministic_and_contains_required_files() {
        let first = zip_bytes(&case()).unwrap();
        let second = zip_bytes(&case()).unwrap();
        assert_eq!(first, second);
        let archive = zip::ZipArchive::new(Cursor::new(first)).unwrap();
        let names: Vec<_> = archive.file_names().map(str::to_owned).collect();
        for required in [
            "summary.html",
            "findings.json",
            "system-profile.json",
            "collection-manifest.json",
            "redaction-manifest.json",
            "checksums.sha256",
        ] {
            assert!(names.contains(&required.to_owned()));
        }
    }
}
