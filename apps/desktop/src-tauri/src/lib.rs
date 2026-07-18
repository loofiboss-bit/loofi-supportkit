use chrono::Utc;
use collector_runtime::collect_current_platform;
use redaction_engine::{RedactionOptions, redact};
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use supportkit_core::{
    CASE_SCHEMA_VERSION, ExportFormat, ExportRecord, ProblemCategory, ScanStatus, SupportCase,
    bundled_rules, evaluate_rules,
};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicError {
    code: &'static str,
    message: String,
}

impl PublicError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

type CommandResult<T> = Result<T, PublicError>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CaseSummary {
    case_id: Uuid,
    created_at: chrono::DateTime<Utc>,
    platform: supportkit_core::Platform,
    category: ProblemCategory,
    scan_status: ScanStatus,
    finding_count: usize,
    highest_severity: Option<supportkit_core::Severity>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportResult {
    destination: String,
    format: ExportFormat,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StorageInfo {
    case_directory: String,
    export_directory: String,
    raw_evidence_persisted: bool,
}

#[tauri::command]
fn run_scan(
    app: AppHandle,
    category: ProblemCategory,
    redact_ssids: bool,
) -> CommandResult<SupportCase> {
    let created_at = Utc::now();
    let batch = collect_current_platform(category, created_at)
        .map_err(|error| PublicError::new("unsupported_platform", error.to_string()))?;
    let findings = evaluate_rules(
        &bundled_rules().map_err(|_| {
            PublicError::new("rules_unavailable", "Diagnostic rules could not be loaded.")
        })?,
        &batch.evidence,
        batch.platform,
        category,
    );
    let raw = serde_json::json!({
        "systemProfile": batch.profile,
        "collectionManifest": batch.records,
        "evidence": batch.evidence,
    });
    let redacted = redact(&raw, RedactionOptions { redact_ssids });
    let system_profile =
        serde_json::from_value(redacted.value["systemProfile"].clone()).map_err(|_| {
            PublicError::new(
                "redaction_failed",
                "The privacy-reviewed system profile could not be created.",
            )
        })?;
    let evidence = serde_json::from_value(redacted.value["evidence"].clone()).map_err(|_| {
        PublicError::new(
            "redaction_failed",
            "The privacy-reviewed evidence could not be created.",
        )
    })?;
    let collection_manifest: Vec<supportkit_core::CollectorRecord> =
        serde_json::from_value(redacted.value["collectionManifest"].clone()).map_err(|_| {
            PublicError::new(
                "redaction_failed",
                "The privacy-reviewed collection manifest could not be created.",
            )
        })?;
    let partial = collection_manifest
        .iter()
        .any(|record| !matches!(record.status, supportkit_core::CollectorState::Complete));
    let case = SupportCase {
        schema_version: CASE_SCHEMA_VERSION.into(),
        case_id: Uuid::new_v4(),
        created_at,
        app_version: env!("CARGO_PKG_VERSION").into(),
        platform: batch.platform,
        category,
        scan_status: if partial {
            ScanStatus::Partial
        } else {
            ScanStatus::Complete
        },
        system_profile,
        collection_manifest,
        findings,
        evidence,
        redaction_manifest: redacted.manifest,
        export_history: vec![],
    };
    save_case(&app, &case)?;
    Ok(case)
}

#[tauri::command]
fn list_cases(app: AppHandle) -> CommandResult<Vec<CaseSummary>> {
    let directory = case_directory(&app)?;
    fs::create_dir_all(&directory).map_err(|_| {
        PublicError::new(
            "storage_unavailable",
            "The local case directory is unavailable.",
        )
    })?;
    let mut summaries = Vec::new();
    let entries = fs::read_dir(directory)
        .map_err(|_| PublicError::new("storage_unavailable", "Local cases could not be listed."))?;
    for entry in entries.flatten() {
        if entry.path().extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let Ok(case) = read_case_path(&entry.path()) else {
            continue;
        };
        summaries.push(CaseSummary {
            case_id: case.case_id,
            created_at: case.created_at,
            platform: case.platform,
            category: case.category,
            scan_status: case.scan_status,
            finding_count: case.findings.len(),
            highest_severity: case.findings.first().map(|finding| finding.severity),
        });
    }
    summaries.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    Ok(summaries)
}

#[tauri::command]
fn load_case(app: AppHandle, case_id: String) -> CommandResult<SupportCase> {
    let id = parse_case_id(&case_id)?;
    let path = case_directory(&app)?.join(format!("{id}.json"));
    read_case_path(&path)
}

#[tauri::command]
fn delete_case(app: AppHandle, case_id: String) -> CommandResult<()> {
    let id = parse_case_id(&case_id)?;
    let source = case_directory(&app)?.join(format!("{id}.json"));
    if !source.is_file() {
        return Err(PublicError::new(
            "case_not_found",
            "The selected case no longer exists.",
        ));
    }
    let trash = app_root(&app)?.join("trash");
    fs::create_dir_all(&trash).map_err(|_| {
        PublicError::new(
            "storage_unavailable",
            "The recovery directory is unavailable.",
        )
    })?;
    fs::rename(source, trash.join(format!("{id}.json"))).map_err(|_| {
        PublicError::new(
            "delete_failed",
            "The case could not be moved to the recovery directory.",
        )
    })
}

#[tauri::command]
fn export_case(
    app: AppHandle,
    case_id: String,
    format: ExportFormat,
) -> CommandResult<ExportResult> {
    let id = parse_case_id(&case_id)?;
    let mut case = read_case_path(&case_directory(&app)?.join(format!("{id}.json")))?;
    let exports = export_directory(&app)?;
    fs::create_dir_all(&exports).map_err(|_| {
        PublicError::new(
            "export_unavailable",
            "The local export directory is unavailable.",
        )
    })?;
    let extension = match format {
        ExportFormat::Zip => "zip",
        ExportFormat::Html => "html",
        ExportFormat::Json => "json",
    };
    let name = format!("supportkit-case-{}.{}", case.case_id, extension);
    let destination = exports.join(&name);
    case.export_history.push(ExportRecord {
        exported_at: Utc::now(),
        format,
        destination_name: name,
    });
    let result = match format {
        ExportFormat::Zip => report_builder::write_zip(&case, &destination),
        ExportFormat::Html => report_builder::write_html(&case, &destination),
        ExportFormat::Json => report_builder::write_json(&case, &destination),
    };
    result.map_err(|_| {
        PublicError::new("export_failed", "The support case could not be exported.")
    })?;
    save_case(&app, &case)?;
    Ok(ExportResult {
        destination: destination.display().to_string(),
        format,
    })
}

#[tauri::command]
fn get_storage_info(app: AppHandle) -> CommandResult<StorageInfo> {
    Ok(StorageInfo {
        case_directory: case_directory(&app)?.display().to_string(),
        export_directory: export_directory(&app)?.display().to_string(),
        raw_evidence_persisted: false,
    })
}

fn app_root(app: &AppHandle) -> CommandResult<PathBuf> {
    app.path()
        .app_data_dir()
        .map(|path| path.join("supportkit"))
        .map_err(|_| {
            PublicError::new(
                "storage_unavailable",
                "The application data directory is unavailable.",
            )
        })
}

fn case_directory(app: &AppHandle) -> CommandResult<PathBuf> {
    Ok(app_root(app)?.join("cases"))
}

fn export_directory(app: &AppHandle) -> CommandResult<PathBuf> {
    Ok(app_root(app)?.join("exports"))
}

fn parse_case_id(case_id: &str) -> CommandResult<Uuid> {
    Uuid::parse_str(case_id)
        .map_err(|_| PublicError::new("invalid_case_id", "The case identifier is invalid."))
}

fn read_case_path(path: &Path) -> CommandResult<SupportCase> {
    let bytes = fs::read(path)
        .map_err(|_| PublicError::new("case_not_found", "The selected case could not be read."))?;
    let case: SupportCase = serde_json::from_slice(&bytes).map_err(|_| {
        PublicError::new(
            "case_invalid",
            "The selected case is corrupt or uses an unsupported schema.",
        )
    })?;
    if case.schema_version != CASE_SCHEMA_VERSION {
        return Err(PublicError::new(
            "schema_unsupported",
            "The selected case uses an unsupported schema version.",
        ));
    }
    Ok(case)
}

fn save_case(app: &AppHandle, case: &SupportCase) -> CommandResult<()> {
    let directory = case_directory(app)?;
    fs::create_dir_all(&directory).map_err(|_| {
        PublicError::new(
            "storage_unavailable",
            "The local case directory is unavailable.",
        )
    })?;
    let destination = directory.join(format!("{}.json", case.case_id));
    atomic_write_json(&destination, case)
}

fn atomic_write_json(path: &Path, case: &SupportCase) -> CommandResult<()> {
    let bytes = serde_json::to_vec_pretty(case)
        .map_err(|_| PublicError::new("storage_failed", "The case could not be serialized."))?;
    let temporary = path.with_extension(format!("tmp-{}", Uuid::new_v4()));
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary).map_err(|_| {
        PublicError::new(
            "storage_failed",
            "A temporary case file could not be created.",
        )
    })?;
    let write_result = (|| -> std::io::Result<()> {
        file.write_all(&bytes)?;
        file.write_all(b"\n")?;
        file.sync_all()
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary);
        return Err(PublicError::new(
            "storage_failed",
            "The local case write did not complete.",
        ));
    }
    fs::rename(&temporary, path).map_err(|_| {
        PublicError::new(
            "storage_failed",
            "The local case could not be committed atomically.",
        )
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            run_scan,
            list_cases,
            load_case,
            delete_case,
            export_case,
            get_storage_info,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Loofi SupportKit");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_case_round_trip_preserves_the_supported_schema() {
        let support_case: SupportCase =
            serde_json::from_str(include_str!("../../../../fixtures/support-case.json"))
                .expect("fixture must deserialize");
        let directory = std::env::temp_dir().join(format!("supportkit-test-{}", Uuid::new_v4()));
        fs::create_dir(&directory).expect("temporary directory must be created");
        let path = directory.join("case.json");

        atomic_write_json(&path, &support_case).expect("atomic write must succeed");
        let reopened = read_case_path(&path).expect("saved case must reopen");

        assert_eq!(reopened.case_id, support_case.case_id);
        assert_eq!(reopened.schema_version, CASE_SCHEMA_VERSION);
        fs::remove_dir_all(directory).expect("temporary directory must be removed");
    }
}
