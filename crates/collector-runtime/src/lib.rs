use chrono::{DateTime, Utc};
#[cfg(target_os = "linux")]
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[cfg(target_os = "linux")]
use std::fs;
use std::io::Read;
#[cfg(target_os = "linux")]
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use supportkit_core::{
    CollectorRecord, CollectorState, Evidence, Platform, ProblemCategory, SystemProfile,
};
use thiserror::Error;

const OUTPUT_LIMIT: usize = 256 * 1024;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionBatch {
    pub platform: Platform,
    pub profile: SystemProfile,
    pub evidence: Vec<Evidence>,
    pub records: Vec<CollectorRecord>,
}

#[derive(Debug, Error)]
pub enum CollectionError {
    #[error("unsupported platform: this release supports Windows 11 x64 and Fedora KDE 44 x64")]
    UnsupportedPlatform,
}

#[derive(Debug, Error)]
enum RunnerError {
    #[error("allowlisted executable is unavailable: {0}")]
    ExecutableUnavailable(&'static str),
    #[error("failed to start allowlisted executable: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("process output reader failed")]
    OutputReader,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Executable {
    #[cfg(target_os = "linux")]
    Df,
    #[cfg(target_os = "linux")]
    Systemctl,
    #[cfg(target_os = "linux")]
    Journalctl,
    #[cfg(target_os = "linux")]
    Dnf,
    #[cfg(target_os = "linux")]
    Nmcli,
    #[cfg(target_os = "linux")]
    Lspci,
    #[cfg(target_os = "linux")]
    PlasmaShell,
    #[cfg(target_os = "windows")]
    PowerShell,
}

impl Executable {
    fn candidates(self) -> &'static [&'static str] {
        match self {
            #[cfg(target_os = "linux")]
            Self::Df => &["/usr/bin/df"],
            #[cfg(target_os = "linux")]
            Self::Systemctl => &["/usr/bin/systemctl"],
            #[cfg(target_os = "linux")]
            Self::Journalctl => &["/usr/bin/journalctl"],
            #[cfg(target_os = "linux")]
            Self::Dnf => &["/usr/bin/dnf5", "/usr/bin/dnf"],
            #[cfg(target_os = "linux")]
            Self::Nmcli => &["/usr/bin/nmcli"],
            #[cfg(target_os = "linux")]
            Self::Lspci => &["/usr/bin/lspci"],
            #[cfg(target_os = "linux")]
            Self::PlasmaShell => &["/usr/bin/plasmashell"],
            #[cfg(target_os = "windows")]
            Self::PowerShell => &[r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe"],
        }
    }

    fn label(self) -> &'static str {
        match self {
            #[cfg(target_os = "linux")]
            Self::Df => "df",
            #[cfg(target_os = "linux")]
            Self::Systemctl => "systemctl",
            #[cfg(target_os = "linux")]
            Self::Journalctl => "journalctl",
            #[cfg(target_os = "linux")]
            Self::Dnf => "dnf",
            #[cfg(target_os = "linux")]
            Self::Nmcli => "nmcli",
            #[cfg(target_os = "linux")]
            Self::Lspci => "lspci",
            #[cfg(target_os = "linux")]
            Self::PlasmaShell => "plasmashell",
            #[cfg(target_os = "windows")]
            Self::PowerShell => "powershell.exe",
        }
    }
}

struct CommandSpec {
    executable: Executable,
    arguments: Vec<String>,
    timeout: Duration,
    accepted_codes: &'static [i32],
}

impl CommandSpec {
    fn fixed(executable: Executable, arguments: &[&str]) -> Self {
        Self {
            executable,
            arguments: arguments.iter().map(|value| (*value).to_owned()).collect(),
            timeout: DEFAULT_TIMEOUT,
            accepted_codes: &[0],
        }
    }
}

struct ProcessOutcome {
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    timed_out: bool,
    truncated: bool,
    duration_ms: u64,
    accepted_codes: &'static [i32],
}

impl ProcessOutcome {
    fn succeeded(&self) -> bool {
        self.exit_code
            .is_some_and(|code| self.accepted_codes.contains(&code))
            && !self.timed_out
    }
}

fn resolve_executable(executable: Executable) -> Result<PathBuf, RunnerError> {
    executable
        .candidates()
        .iter()
        .map(PathBuf::from)
        .find(|path| path.is_file())
        .ok_or_else(|| RunnerError::ExecutableUnavailable(executable.label()))
}

fn run_process(spec: CommandSpec) -> Result<ProcessOutcome, RunnerError> {
    let executable = resolve_executable(spec.executable)?;
    let started = Instant::now();
    let mut child = Command::new(executable)
        .args(&spec.arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear()
        .env("LANG", "C.UTF-8")
        .env("LC_ALL", "C.UTF-8")
        .env("PATH", "/usr/bin:/bin")
        .spawn()?;

    let stdout = child.stdout.take().ok_or(RunnerError::OutputReader)?;
    let stderr = child.stderr.take().ok_or(RunnerError::OutputReader)?;
    let stdout_reader = thread::spawn(move || read_limited(stdout));
    let stderr_reader = thread::spawn(move || read_limited(stderr));

    let (exit_code, timed_out) = loop {
        if let Some(status) = child.try_wait()? {
            break (status.code(), false);
        }
        if started.elapsed() >= spec.timeout {
            let _ = child.kill();
            let status = child.wait()?;
            break (status.code(), true);
        }
        thread::sleep(Duration::from_millis(20));
    };

    let (stdout, stdout_truncated) = stdout_reader
        .join()
        .map_err(|_| RunnerError::OutputReader)??;
    let (stderr, stderr_truncated) = stderr_reader
        .join()
        .map_err(|_| RunnerError::OutputReader)??;
    Ok(ProcessOutcome {
        exit_code,
        stdout,
        stderr,
        timed_out,
        truncated: stdout_truncated || stderr_truncated,
        duration_ms: started.elapsed().as_millis().try_into().unwrap_or(u64::MAX),
        accepted_codes: spec.accepted_codes,
    })
}

fn read_limited(mut reader: impl Read) -> std::io::Result<(String, bool)> {
    let mut bytes = Vec::with_capacity(OUTPUT_LIMIT.min(8192));
    reader
        .by_ref()
        .take((OUTPUT_LIMIT + 1) as u64)
        .read_to_end(&mut bytes)?;
    let truncated = bytes.len() > OUTPUT_LIMIT;
    bytes.truncate(OUTPUT_LIMIT);
    Ok((String::from_utf8_lossy(&bytes).into_owned(), truncated))
}

pub fn collect_current_platform(
    category: ProblemCategory,
    collected_at: DateTime<Utc>,
) -> Result<CollectionBatch, CollectionError> {
    #[cfg(target_os = "linux")]
    {
        if fedora_version().as_deref() == Some("44") && std::env::consts::ARCH == "x86_64" {
            return Ok(collect_fedora(category, collected_at));
        }
    }
    #[cfg(target_os = "windows")]
    {
        if std::env::consts::ARCH == "x86_64" {
            return Ok(collect_windows(category, collected_at));
        }
    }
    Err(CollectionError::UnsupportedPlatform)
}

#[cfg(target_os = "linux")]
fn fedora_version() -> Option<String> {
    let content = fs::read_to_string("/etc/os-release").ok()?;
    let id = value_from_os_release(&content, "ID")?;
    (id == "fedora")
        .then(|| value_from_os_release(&content, "VERSION_ID"))
        .flatten()
}

#[cfg(target_os = "linux")]
fn value_from_os_release(content: &str, key: &str) -> Option<String> {
    content.lines().find_map(|line| {
        let (name, value) = line.split_once('=')?;
        (name == key).then(|| value.trim_matches('"').to_owned())
    })
}

#[cfg(target_os = "linux")]
fn collect_fedora(category: ProblemCategory, collected_at: DateTime<Utc>) -> CollectionBatch {
    let os_release = fs::read_to_string("/etc/os-release").unwrap_or_default();
    let plasma = run_process(CommandSpec::fixed(Executable::PlasmaShell, &["--version"]))
        .ok()
        .filter(ProcessOutcome::succeeded)
        .map(|output| output.stdout.trim().to_owned());
    let profile = SystemProfile {
        os_name: value_from_os_release(&os_release, "NAME")
            .unwrap_or_else(|| "Fedora Linux".into()),
        os_version: value_from_os_release(&os_release, "VERSION_ID").unwrap_or_else(|| "44".into()),
        architecture: std::env::consts::ARCH.into(),
        desktop: plasma,
        hostname: fs::read_to_string("/etc/hostname")
            .ok()
            .map(|value| value.trim().to_owned()),
    };
    let mut batch = CollectionBatch {
        platform: Platform::FedoraKde44X64,
        profile: profile.clone(),
        evidence: vec![Evidence {
            id: "evidence.system.profile".into(),
            collector_id: "fedora.system-profile".into(),
            kind: "system_profile".into(),
            collected_at,
            data: json!({
                "osName": profile.os_name,
                "osVersion": profile.os_version,
                "architecture": profile.architecture,
                "desktop": profile.desktop,
                "hostname": profile.hostname,
                "kernel": fs::read_to_string("/proc/sys/kernel/osrelease").ok().map(|value| value.trim().to_owned()),
                "sessionType": std::env::var("XDG_SESSION_TYPE").ok(),
            }),
        }],
        records: vec![record(
            "fedora.system-profile",
            CollectorState::Complete,
            None,
            0,
            false,
        )],
    };

    if matches!(
        category,
        ProblemCategory::General | ProblemCategory::Storage | ProblemCategory::Performance
    ) {
        add_process_evidence(
            &mut batch,
            "common.disk-usage",
            "evidence.storage.root",
            "disk_usage",
            CommandSpec::fixed(
                Executable::Df,
                &["-B1", "--output=source,size,used,avail,pcent,target", "/"],
            ),
            collected_at,
            parse_disk_usage,
        );
    }
    if matches!(
        category,
        ProblemCategory::General | ProblemCategory::Services | ProblemCategory::Performance
    ) {
        collect_failed_units(&mut batch, collected_at);
    }
    if matches!(
        category,
        ProblemCategory::General | ProblemCategory::Updates
    ) {
        let mut spec = CommandSpec::fixed(Executable::Dnf, &["check-upgrade", "--quiet"]);
        spec.timeout = Duration::from_secs(15);
        spec.accepted_codes = &[0, 100];
        add_process_evidence(
            &mut batch,
            "fedora.update-state",
            "evidence.updates.state",
            "update_state",
            spec,
            collected_at,
            parse_updates,
        );
    }
    if matches!(
        category,
        ProblemCategory::General | ProblemCategory::Network
    ) {
        let spec = CommandSpec::fixed(
            Executable::Nmcli,
            &[
                "-t",
                "-f",
                "DEVICE,TYPE,STATE,CONNECTION",
                "device",
                "status",
            ],
        );
        add_process_evidence(
            &mut batch,
            "fedora.network-state",
            "evidence.network.state",
            "network_state",
            spec,
            collected_at,
            parse_network,
        );
        add_native_evidence(
            &mut batch,
            "fedora.dns",
            "evidence.network.dns",
            "dns_configuration",
            collected_at,
            dns_native(),
        );
    }
    if matches!(
        category,
        ProblemCategory::General | ProblemCategory::DriversGraphics
    ) {
        let spec = CommandSpec::fixed(Executable::Lspci, &["-mm", "-nn", "-k"]);
        add_process_evidence(
            &mut batch,
            "fedora.gpu",
            "evidence.graphics.gpu",
            "gpu_driver",
            spec,
            collected_at,
            |output| json!({ "pciDevices": output.lines().filter(|line| line.to_ascii_lowercase().contains("vga") || line.to_ascii_lowercase().contains("display")).take(16).collect::<Vec<_>>(), "loadedModules": loaded_modules() }),
        );
    }
    batch
}

fn add_process_evidence<F>(
    batch: &mut CollectionBatch,
    collector_id: &str,
    evidence_id: &str,
    kind: &str,
    spec: CommandSpec,
    collected_at: DateTime<Utc>,
    parser: F,
) where
    F: FnOnce(&str) -> Value,
{
    match run_process(spec) {
        Ok(outcome) if outcome.succeeded() => {
            batch.evidence.push(Evidence {
                id: evidence_id.into(),
                collector_id: collector_id.into(),
                kind: kind.into(),
                collected_at,
                data: parser(&outcome.stdout),
            });
            batch.records.push(record(
                collector_id,
                CollectorState::Complete,
                nonempty(&outcome.stderr),
                outcome.duration_ms,
                outcome.truncated,
            ));
        }
        Ok(outcome) => {
            let state = if outcome.timed_out {
                CollectorState::TimedOut
            } else {
                CollectorState::Failed
            };
            let message = if outcome.timed_out {
                Some("Collector exceeded its time limit.".into())
            } else {
                Some(format!(
                    "Collector exited with status {:?}: {}",
                    outcome.exit_code,
                    bounded_message(&outcome.stderr)
                ))
            };
            batch.records.push(record(
                collector_id,
                state,
                message,
                outcome.duration_ms,
                outcome.truncated,
            ));
        }
        Err(error) => batch.records.push(record(
            collector_id,
            CollectorState::Skipped,
            Some(error.to_string()),
            0,
            false,
        )),
    }
}

#[cfg(target_os = "linux")]
fn add_native_evidence(
    batch: &mut CollectionBatch,
    collector_id: &str,
    evidence_id: &str,
    kind: &str,
    collected_at: DateTime<Utc>,
    data: Value,
) {
    batch.evidence.push(Evidence {
        id: evidence_id.into(),
        collector_id: collector_id.into(),
        kind: kind.into(),
        collected_at,
        data,
    });
    batch.records.push(record(
        collector_id,
        CollectorState::Complete,
        None,
        0,
        false,
    ));
}

fn record(
    id: &str,
    status: CollectorState,
    message: Option<String>,
    duration_ms: u64,
    output_truncated: bool,
) -> CollectorRecord {
    CollectorRecord {
        collector_id: id.into(),
        status,
        message,
        duration_ms,
        output_truncated,
    }
}

fn nonempty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| bounded_message(trimmed))
}

fn bounded_message(value: &str) -> String {
    value.chars().take(400).collect()
}

#[cfg(target_os = "linux")]
fn parse_disk_usage(output: &str) -> Value {
    let fields: Vec<_> = output
        .lines()
        .last()
        .unwrap_or_default()
        .split_whitespace()
        .collect();
    let used_percent = fields
        .get(4)
        .and_then(|value| value.trim_end_matches('%').parse::<u64>().ok())
        .unwrap_or(0);
    let available_bytes = fields
        .get(3)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    json!({ "device": fields.first(), "mount": fields.get(5).copied().unwrap_or("/"), "usedPercent": used_percent, "availableBytes": available_bytes })
}

#[cfg(target_os = "linux")]
fn parse_failed_unit_names(output: &str) -> Vec<String> {
    let unit_pattern =
        Regex::new(r"^[A-Za-z0-9@_.:-]{1,200}\.(?:service|socket|mount|target|timer)$")
            .expect("valid unit regex");
    output
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| unit_pattern.is_match(name))
        .map(str::to_owned)
        .take(50)
        .collect()
}

#[cfg(target_os = "linux")]
fn collect_failed_units(batch: &mut CollectionBatch, collected_at: DateTime<Utc>) {
    let spec = CommandSpec::fixed(
        Executable::Systemctl,
        &["--failed", "--no-legend", "--plain", "--no-pager"],
    );
    let outcome = match run_process(spec) {
        Ok(outcome) if outcome.succeeded() => outcome,
        Ok(outcome) => {
            let state = if outcome.timed_out {
                CollectorState::TimedOut
            } else {
                CollectorState::Failed
            };
            batch.records.push(record(
                "fedora.failed-units",
                state,
                Some("Failed-unit state could not be collected.".into()),
                outcome.duration_ms,
                outcome.truncated,
            ));
            return;
        }
        Err(error) => {
            batch.records.push(record(
                "fedora.failed-units",
                CollectorState::Skipped,
                Some(error.to_string()),
                0,
                false,
            ));
            return;
        }
    };
    let units = parse_failed_unit_names(&outcome.stdout);
    batch.evidence.push(Evidence {
        id: "evidence.services.failed".into(),
        collector_id: "fedora.failed-units".into(),
        kind: "failed_services".into(),
        collected_at,
        data: json!({ "failedCount": units.len(), "units": units.clone() }),
    });
    batch.records.push(record(
        "fedora.failed-units",
        CollectorState::Complete,
        nonempty(&outcome.stderr),
        outcome.duration_ms,
        outcome.truncated,
    ));

    if units.is_empty() {
        batch.records.push(record(
            "fedora.failed-unit-journal",
            CollectorState::Complete,
            Some("No failed units required journal collection.".into()),
            0,
            false,
        ));
        return;
    }
    let mut journals = Vec::new();
    let mut total_duration = 0_u64;
    let mut truncated = false;
    let mut failures = 0_u64;
    for unit in units.iter().take(5) {
        let spec = CommandSpec {
            executable: Executable::Journalctl,
            arguments: vec![
                "--unit".into(),
                unit.clone(),
                "--priority".into(),
                "warning..alert".into(),
                "--no-pager".into(),
                "--lines".into(),
                "30".into(),
                "--output".into(),
                "json".into(),
            ],
            timeout: DEFAULT_TIMEOUT,
            accepted_codes: &[0],
        };
        match run_process(spec) {
            Ok(result) if result.succeeded() => {
                total_duration = total_duration.saturating_add(result.duration_ms);
                truncated |= result.truncated;
                let events: Vec<Value> = result
                    .stdout
                    .lines()
                    .take(30)
                    .filter_map(|line| serde_json::from_str(line).ok())
                    .collect();
                journals.push(json!({ "unit": unit, "events": events }));
            }
            Ok(result) => {
                total_duration = total_duration.saturating_add(result.duration_ms);
                truncated |= result.truncated;
                failures += 1;
            }
            Err(_) => failures += 1,
        }
    }
    if !journals.is_empty() {
        batch.evidence.push(Evidence {
            id: "evidence.services.failed-journal".into(),
            collector_id: "fedora.failed-unit-journal".into(),
            kind: "failed_service_journal".into(),
            collected_at,
            data: json!({ "units": journals }),
        });
    }
    batch.records.push(record(
        "fedora.failed-unit-journal",
        if failures == 0 {
            CollectorState::Complete
        } else if journals.is_empty() {
            CollectorState::Failed
        } else {
            CollectorState::Partial
        },
        (failures > 0)
            .then(|| format!("Journal evidence was unavailable for {failures} failed unit(s).")),
        total_duration,
        truncated,
    ));
}

#[cfg(target_os = "linux")]
fn parse_updates(output: &str) -> Value {
    let pending = output
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty()
                && !trimmed.starts_with("Updating")
                && trimmed
                    .split_whitespace()
                    .next()
                    .is_some_and(|field| field.contains('.'))
        })
        .take(10_000)
        .count();
    json!({ "pendingCount": pending, "rebootPending": Path::new("/run/reboot-required").exists() })
}

#[cfg(target_os = "linux")]
fn parse_network(output: &str) -> Value {
    let connections: Vec<_> = output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(64)
        .collect();
    let connected = connections.iter().any(|line| line.contains(":connected:"));
    json!({ "connected": connected, "connections": connections })
}

#[cfg(target_os = "linux")]
fn dns_native() -> Value {
    let content = fs::read_to_string("/etc/resolv.conf").unwrap_or_default();
    let servers: Vec<_> = content
        .lines()
        .filter_map(|line| line.strip_prefix("nameserver "))
        .map(str::trim)
        .take(8)
        .collect();
    json!({ "servers": servers, "source": "/etc/resolv.conf" })
}

#[cfg(target_os = "linux")]
fn loaded_modules() -> Vec<String> {
    fs::read_to_string("/proc/modules")
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.split_whitespace().next().map(str::to_owned))
        .take(512)
        .collect()
}

#[cfg(target_os = "windows")]
fn collect_windows(category: ProblemCategory, collected_at: DateTime<Utc>) -> CollectionBatch {
    const PROFILE: &str = "$o=Get-CimInstance Win32_OperatingSystem;[pscustomobject]@{caption=$o.Caption;version=$o.Version;build=$o.BuildNumber;architecture=$o.OSArchitecture;hostname=$env:COMPUTERNAME}|ConvertTo-Json -Compress";
    let profile_data = powershell_json(PROFILE).unwrap_or_else(|| json!({}));
    let profile = SystemProfile {
        os_name: profile_data
            .get("caption")
            .and_then(Value::as_str)
            .unwrap_or("Windows 11")
            .into(),
        os_version: profile_data
            .get("build")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .into(),
        architecture: std::env::consts::ARCH.into(),
        desktop: None,
        hostname: profile_data
            .get("hostname")
            .and_then(Value::as_str)
            .map(str::to_owned),
    };
    let mut batch = CollectionBatch {
        platform: Platform::Windows11X64,
        profile: profile.clone(),
        evidence: vec![Evidence {
            id: "evidence.system.profile".into(),
            collector_id: "windows.system-profile".into(),
            kind: "system_profile".into(),
            collected_at,
            data: profile_data,
        }],
        records: vec![record(
            "windows.system-profile",
            CollectorState::Complete,
            None,
            0,
            false,
        )],
    };
    let plans: Vec<(&str, &str, &str, &str)> = match category {
        ProblemCategory::Storage => vec![(
            "common.disk-usage",
            "evidence.storage.system",
            "disk_usage",
            "$d=Get-CimInstance Win32_LogicalDisk -Filter \"DeviceID='$env:SystemDrive'\";[pscustomobject]@{mount=$d.DeviceID;usedPercent=[math]::Round((1-($d.FreeSpace/$d.Size))*100);availableBytes=$d.FreeSpace}|ConvertTo-Json -Compress",
        )],
        ProblemCategory::Services => vec![(
            "windows.service-state",
            "evidence.services.stopped",
            "service_state",
            "$s=@(Get-CimInstance Win32_Service|Where-Object{$_.StartMode -eq 'Auto' -and $_.State -ne 'Running'});[pscustomobject]@{stoppedAutomaticCount=$s.Count;services=@($s|Select-Object -First 50 Name,State)}|ConvertTo-Json -Depth 3 -Compress",
        )],
        _ => vec![
            (
                "common.disk-usage",
                "evidence.storage.system",
                "disk_usage",
                "$d=Get-CimInstance Win32_LogicalDisk -Filter \"DeviceID='$env:SystemDrive'\";[pscustomobject]@{mount=$d.DeviceID;usedPercent=[math]::Round((1-($d.FreeSpace/$d.Size))*100);availableBytes=$d.FreeSpace}|ConvertTo-Json -Compress",
            ),
            (
                "windows.service-state",
                "evidence.services.stopped",
                "service_state",
                "$s=@(Get-CimInstance Win32_Service|Where-Object{$_.StartMode -eq 'Auto' -and $_.State -ne 'Running'});[pscustomobject]@{stoppedAutomaticCount=$s.Count;services=@($s|Select-Object -First 50 Name,State)}|ConvertTo-Json -Depth 3 -Compress",
            ),
            (
                "windows.pnp",
                "evidence.devices.errors",
                "pnp_errors",
                "Get-CimInstance Win32_PnPEntity|Where-Object{$_.ConfigManagerErrorCode -ne 0}|Select-Object -First 50 Name,PNPClass,ConfigManagerErrorCode|ConvertTo-Json -Compress",
            ),
            (
                "windows.network",
                "evidence.network.adapters",
                "network_state",
                "Get-NetAdapter|Select-Object Name,InterfaceDescription,Status,LinkSpeed,MacAddress|ConvertTo-Json -Compress",
            ),
            (
                "windows.gpu",
                "evidence.graphics.gpu",
                "gpu_driver",
                "Get-CimInstance Win32_VideoController|Select-Object Name,DriverVersion,DriverDate,Status|ConvertTo-Json -Compress",
            ),
            (
                "windows.updates",
                "evidence.updates.state",
                "update_state",
                "$r=(Get-ItemProperty 'HKLM:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\WindowsUpdate\\Auto Update\\RebootRequired' -ErrorAction SilentlyContinue);[pscustomobject]@{rebootPending=($null-ne $r)}|ConvertTo-Json -Compress",
            ),
            (
                "windows.events",
                "evidence.events.recent",
                "event_log",
                "Get-WinEvent -FilterHashtable @{LogName='System';Level=1,2;StartTime=(Get-Date).AddDays(-3)} -MaxEvents 50 -ErrorAction SilentlyContinue|Select-Object TimeCreated,Id,LevelDisplayName,ProviderName,Message|ConvertTo-Json -Compress",
            ),
        ],
    };
    for (collector, id, kind, script) in plans {
        let mut spec = CommandSpec::fixed(
            Executable::PowerShell,
            &[
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                script,
            ],
        );
        spec.timeout = Duration::from_secs(12);
        add_process_evidence(
            &mut batch,
            collector,
            id,
            kind,
            spec,
            collected_at,
            |output| serde_json::from_str(output).unwrap_or_else(|_| json!({ "parseError": true })),
        );
    }
    batch
}

#[cfg(target_os = "windows")]
fn powershell_json(script: &'static str) -> Option<Value> {
    let spec = CommandSpec::fixed(
        Executable::PowerShell,
        &[
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ],
    );
    run_process(spec)
        .ok()
        .filter(ProcessOutcome::succeeded)
        .and_then(|outcome| serde_json::from_str(&outcome.stdout).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    #[test]
    fn executable_allowlist_contains_no_shells() {
        for executable in [
            Executable::Df,
            Executable::Systemctl,
            Executable::Journalctl,
            Executable::Dnf,
            Executable::Nmcli,
            Executable::Lspci,
            Executable::PlasmaShell,
        ] {
            for candidate in executable.candidates() {
                assert!(!candidate.ends_with("/sh"));
                assert!(!candidate.ends_with("cmd.exe"));
            }
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn failed_unit_parser_bounds_and_validates_names() {
        let parsed = parse_failed_unit_names(
            "good.service loaded failed failed\n../../bad.service loaded failed\nnot-a-unit failed\n",
        );
        assert_eq!(parsed, vec!["good.service"]);
    }

    #[test]
    fn limited_reader_truncates_without_invalid_utf8_panics() {
        let input = vec![b'a'; OUTPUT_LIMIT + 20];
        let (text, truncated) = read_limited(input.as_slice()).unwrap();
        assert!(truncated);
        assert_eq!(text.len(), OUTPUT_LIMIT);
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "requires a supported live Fedora host"]
    fn live_general_collection_is_fail_soft() {
        let batch = collect_current_platform(ProblemCategory::General, Utc::now())
            .expect("Fedora collection should be available on Linux");
        assert_eq!(batch.platform, Platform::FedoraKde44X64);
        assert!(!batch.profile.os_name.trim().is_empty());
        assert!(!batch.evidence.is_empty());
        assert!(!batch.records.is_empty());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn os_release_parser_does_not_accept_prefixes() {
        let source = "ID_LIKE=fedora\nID=fedora\nVERSION_ID=44\n";
        assert_eq!(
            value_from_os_release(source, "ID").as_deref(),
            Some("fedora")
        );
        assert_eq!(
            value_from_os_release(source, "VERSION_ID").as_deref(),
            Some("44")
        );
    }
}
