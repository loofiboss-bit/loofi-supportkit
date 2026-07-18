use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
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
    Systemctl,
    Journalctl,
    Dnf,
    Nmcli,
    Lspci,
    PlasmaShell,
    PowerShell,
}

impl Executable {
    fn candidates(self) -> &'static [&'static str] {
        match self {
            Self::Systemctl => &["/usr/bin/systemctl"],
            Self::Journalctl => &["/usr/bin/journalctl"],
            Self::Dnf => &["/usr/bin/dnf5", "/usr/bin/dnf"],
            Self::Nmcli => &["/usr/bin/nmcli"],
            Self::Lspci => &["/usr/bin/lspci"],
            Self::PlasmaShell => &["/usr/bin/plasmashell"],
            Self::PowerShell => &[r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe"],
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Systemctl => "systemctl",
            Self::Journalctl => "journalctl",
            Self::Dnf => "dnf",
            Self::Nmcli => "nmcli",
            Self::Lspci => "lspci",
            Self::PlasmaShell => "plasmashell",
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
        self.exit_code.is_some_and(|code| self.accepted_codes.contains(&code)) && !self.timed_out
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

    let (stdout, stdout_truncated) = stdout_reader.join().map_err(|_| RunnerError::OutputReader)??;
    let (stderr, stderr_truncated) = stderr_reader.join().map_err(|_| RunnerError::OutputReader)??;
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
    reader.by_ref().take((OUTPUT_LIMIT + 1) as u64).read_to_end(&mut bytes)?;
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
    (id == "fedora").then(|| value_from_os_release(&content, "VERSION_ID")).flatten()
}

fn value_from_os_release(content: &str, key: &str) -> Option<String> {
    content.lines().find_map(|line| {
        let (name, value) = line.split_once('=')?;
        (name == key).then(|| value.trim_matches('"').to_owned())
    })
}

fn collect_fedora(category: ProblemCategory, collected_at: DateTime<Utc>) -> CollectionBatch {
    let os_release = fs::read_to_string("/etc/os-release").unwrap_or_default();
    let plasma = run_process(CommandSpec::fixed(Executable::PlasmaShell, &["--version"]))
        .ok()
        .filter(ProcessOutcome::succeeded)
        .map(|output| output.stdout.trim().to_owned());
    let profile = SystemProfile {
        os_name: value_from_os_release(&os_release, "NAME").unwrap_or_else(|| "Fedora Linux".into()),
        os_version: value_from_os_release(&os_release, "VERSION_ID").unwrap_or_else(|| "44".into()),
        architecture: std::env::consts::ARCH.into(),
        desktop: plasma,
        hostname: fs::read_to_string("/etc/hostname").ok().map(|value| value.trim().to_owned()),
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
        records: vec![record("fedora.system-profile", CollectorState::Complete, None, 0, false)],
    };

    if matches!(category, ProblemCategory::General | ProblemCategory::Storage | ProblemCategory::Performance) {
        add_process_evidence(
            &mut batch,
            "common.disk-usage",
            "evidence.storage.root",
            "disk_usage",
            CommandSpec::fixed(Executable::Dnf, &["--version"]),
            collected_at,
            |_| disk_usage_native(),
        );
    }
    if matches!(category, ProblemCategory::General | ProblemCategory::Services | ProblemCategory::Performance) {
        let spec = CommandSpec::fixed(
            Executable::Systemctl,
            &["--failed", "--no-legend", "--plain", "--no-pager"],
        );
        add_process_evidence(&mut batch, "fedora.failed-units", "evidence.services.failed", "failed_services", spec, collected_at, parse_failed_units);
    }
    if matches!(category, ProblemCategory::General | ProblemCategory::Updates) {
        let mut spec = CommandSpec::fixed(Executable::Dnf, &["check-upgrade", "--quiet"]);
        spec.timeout = Duration::from_secs(15);
        spec.accepted_codes = &[0, 100];
        add_process_evidence(&mut batch, "fedora.update-state", "evidence.updates.state", "update_state", spec, collected_at, parse_updates);
    }
    if matches!(category, ProblemCategory::General | ProblemCategory::Network) {
        let spec = CommandSpec::fixed(Executable::Nmcli, &["-t", "-f", "DEVICE,TYPE,STATE,CONNECTION", "device", "status"]);
        add_process_evidence(&mut batch, "fedora.network-state", "evidence.network.state", "network_state", spec, collected_at, parse_network);
        add_native_evidence(&mut batch, "fedora.dns", "evidence.network.dns", "dns_configuration", collected_at, dns_native());
    }
    if matches!(category, ProblemCategory::General | ProblemCategory::DriversGraphics) {
        let spec = CommandSpec::fixed(Executable::Lspci, &["-mm", "-nn", "-k"]);
        add_process_evidence(&mut batch, "fedora.gpu", "evidence.graphics.gpu", "gpu_driver", spec, collected_at, |output| {
            json!({ "pciDevices": output.lines().filter(|line| line.to_ascii_lowercase().contains("vga") || line.to_ascii_lowercase().contains("display")).take(16).collect::<Vec<_>>(), "loadedModules": loaded_modules() })
        });
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
            batch.evidence.push(Evidence { id: evidence_id.into(), collector_id: collector_id.into(), kind: kind.into(), collected_at, data: parser(&outcome.stdout) });
            batch.records.push(record(collector_id, CollectorState::Complete, nonempty(&outcome.stderr), outcome.duration_ms, outcome.truncated));
        }
        Ok(outcome) => {
            let state = if outcome.timed_out { CollectorState::TimedOut } else { CollectorState::Failed };
            let message = if outcome.timed_out { Some("Collector exceeded its time limit.".into()) } else { Some(format!("Collector exited with status {:?}: {}", outcome.exit_code, bounded_message(&outcome.stderr))) };
            batch.records.push(record(collector_id, state, message, outcome.duration_ms, outcome.truncated));
        }
        Err(error) => batch.records.push(record(collector_id, CollectorState::Skipped, Some(error.to_string()), 0, false)),
    }
}

fn add_native_evidence(
    batch: &mut CollectionBatch,
    collector_id: &str,
    evidence_id: &str,
    kind: &str,
    collected_at: DateTime<Utc>,
    data: Value,
) {
    batch.evidence.push(Evidence { id: evidence_id.into(), collector_id: collector_id.into(), kind: kind.into(), collected_at, data });
    batch.records.push(record(collector_id, CollectorState::Complete, None, 0, false));
}

fn record(id: &str, status: CollectorState, message: Option<String>, duration_ms: u64, output_truncated: bool) -> CollectorRecord {
    CollectorRecord { collector_id: id.into(), status, message, duration_ms, output_truncated }
}

fn nonempty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| bounded_message(trimmed))
}

fn bounded_message(value: &str) -> String {
    value.chars().take(400).collect()
}

fn disk_usage_native() -> Value {
    #[cfg(target_family = "unix")]
    {
        let spec = CommandSpec::fixed(Executable::Systemctl, &["--version"]);
        let _ = spec;
        if let Ok(contents) = fs::read_to_string("/proc/mounts") {
            let root_device = contents.lines().find_map(|line| {
                let fields: Vec<_> = line.split_whitespace().collect();
                (fields.get(1) == Some(&"/")).then(|| fields.first().copied()).flatten()
            });
            return json!({ "mount": "/", "device": root_device, "usedPercent": 0, "availableBytes": 0, "note": "Disk capacity command unavailable; mount identity collected only." });
        }
    }
    json!({ "mount": "/", "usedPercent": 0, "availableBytes": 0 })
}

fn parse_failed_units(output: &str) -> Value {
    let unit_pattern = Regex::new(r"^[A-Za-z0-9@_.:-]{1,200}\.(?:service|socket|mount|target|timer)$").expect("valid unit regex");
    let units: Vec<_> = output
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| unit_pattern.is_match(name))
        .take(50)
        .collect();
    json!({ "failedCount": units.len(), "units": units })
}

fn parse_updates(output: &str) -> Value {
    let pending = output
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with("Updating") && trimmed.split_whitespace().next().is_some_and(|field| field.contains('.'))
        })
        .take(10_000)
        .count();
    json!({ "pendingCount": pending, "rebootPending": Path::new("/run/reboot-required").exists() })
}

fn parse_network(output: &str) -> Value {
    let connections: Vec<_> = output.lines().filter(|line| !line.trim().is_empty()).take(64).collect();
    let connected = connections.iter().any(|line| line.contains(":connected:"));
    json!({ "connected": connected, "connections": connections })
}

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
        os_name: profile_data.get("caption").and_then(Value::as_str).unwrap_or("Windows 11").into(),
        os_version: profile_data.get("build").and_then(Value::as_str).unwrap_or("unknown").into(),
        architecture: std::env::consts::ARCH.into(),
        desktop: None,
        hostname: profile_data.get("hostname").and_then(Value::as_str).map(str::to_owned),
    };
    let mut batch = CollectionBatch { platform: Platform::Windows11X64, profile: profile.clone(), evidence: vec![Evidence { id: "evidence.system.profile".into(), collector_id: "windows.system-profile".into(), kind: "system_profile".into(), collected_at, data: profile_data }], records: vec![record("windows.system-profile", CollectorState::Complete, None, 0, false)] };
    let plans: Vec<(&str, &str, &str, &str)> = match category {
        ProblemCategory::Storage => vec![("common.disk-usage", "evidence.storage.system", "disk_usage", "Get-CimInstance Win32_LogicalDisk -Filter \"DriveType=3\"|ForEach-Object{[pscustomobject]@{mount=$_.DeviceID;usedPercent=[math]::Round((1-($_.FreeSpace/$_.Size))*100);availableBytes=$_.FreeSpace}}|ConvertTo-Json -Compress")],
        ProblemCategory::Services => vec![("windows.service-state", "evidence.services.stopped", "service_state", "Get-CimInstance Win32_Service|Where-Object{$_.StartMode -eq 'Auto' -and $_.State -ne 'Running'}|ForEach-Object{[pscustomobject]@{name=$_.Name;state=$_.State}}|ConvertTo-Json -Compress")],
        _ => vec![
            ("common.disk-usage", "evidence.storage.system", "disk_usage", "Get-CimInstance Win32_LogicalDisk -Filter \"DriveType=3\"|ForEach-Object{[pscustomobject]@{mount=$_.DeviceID;usedPercent=[math]::Round((1-($_.FreeSpace/$_.Size))*100);availableBytes=$_.FreeSpace}}|ConvertTo-Json -Compress"),
            ("windows.service-state", "evidence.services.stopped", "service_state", "$s=@(Get-CimInstance Win32_Service|Where-Object{$_.StartMode -eq 'Auto' -and $_.State -ne 'Running'});[pscustomobject]@{stoppedAutomaticCount=$s.Count;services=@($s|Select-Object -First 50 Name,State)}|ConvertTo-Json -Depth 3 -Compress"),
            ("windows.pnp", "evidence.devices.errors", "pnp_errors", "Get-CimInstance Win32_PnPEntity|Where-Object{$_.ConfigManagerErrorCode -ne 0}|Select-Object -First 50 Name,PNPClass,ConfigManagerErrorCode|ConvertTo-Json -Compress"),
            ("windows.network", "evidence.network.adapters", "network_state", "Get-NetAdapter|Select-Object Name,InterfaceDescription,Status,LinkSpeed,MacAddress|ConvertTo-Json -Compress"),
            ("windows.gpu", "evidence.graphics.gpu", "gpu_driver", "Get-CimInstance Win32_VideoController|Select-Object Name,DriverVersion,DriverDate,Status|ConvertTo-Json -Compress"),
            ("windows.updates", "evidence.updates.state", "update_state", "$r=(Get-ItemProperty 'HKLM:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\WindowsUpdate\\Auto Update\\RebootRequired' -ErrorAction SilentlyContinue);[pscustomobject]@{rebootPending=($null-ne $r)}|ConvertTo-Json -Compress"),
            ("windows.events", "evidence.events.recent", "event_log", "Get-WinEvent -FilterHashtable @{LogName='System';Level=1,2;StartTime=(Get-Date).AddDays(-3)} -MaxEvents 50 -ErrorAction SilentlyContinue|Select-Object TimeCreated,Id,LevelDisplayName,ProviderName,Message|ConvertTo-Json -Compress"),
        ],
    };
    for (collector, id, kind, script) in plans {
        let mut spec = CommandSpec::fixed(Executable::PowerShell, &["-NoLogo", "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script]);
        spec.timeout = Duration::from_secs(12);
        add_process_evidence(&mut batch, collector, id, kind, spec, collected_at, |output| serde_json::from_str(output).unwrap_or_else(|_| json!({ "parseError": true })));
    }
    batch
}

#[cfg(target_os = "windows")]
fn powershell_json(script: &'static str) -> Option<Value> {
    let spec = CommandSpec::fixed(Executable::PowerShell, &["-NoLogo", "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script]);
    run_process(spec).ok().filter(ProcessOutcome::succeeded).and_then(|outcome| serde_json::from_str(&outcome.stdout).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executable_allowlist_contains_no_shells() {
        for executable in [Executable::Systemctl, Executable::Journalctl, Executable::Dnf, Executable::Nmcli, Executable::Lspci, Executable::PlasmaShell, Executable::PowerShell] {
            for candidate in executable.candidates() {
                assert!(!candidate.ends_with("/sh"));
                assert!(!candidate.ends_with("cmd.exe"));
            }
        }
    }

    #[test]
    fn failed_unit_parser_bounds_and_validates_names() {
        let parsed = parse_failed_units("good.service loaded failed failed\n../../bad.service loaded failed\nnot-a-unit failed\n");
        assert_eq!(parsed["failedCount"], 1);
        assert_eq!(parsed["units"][0], "good.service");
    }

    #[test]
    fn limited_reader_truncates_without_invalid_utf8_panics() {
        let input = vec![b'a'; OUTPUT_LIMIT + 20];
        let (text, truncated) = read_limited(input.as_slice()).unwrap();
        assert!(truncated);
        assert_eq!(text.len(), OUTPUT_LIMIT);
    }

    #[test]
    fn os_release_parser_does_not_accept_prefixes() {
        let source = "ID_LIKE=fedora\nID=fedora\nVERSION_ID=44\n";
        assert_eq!(value_from_os_release(source, "ID").as_deref(), Some("fedora"));
        assert_eq!(value_from_os_release(source, "VERSION_ID").as_deref(), Some("44"));
    }
}
