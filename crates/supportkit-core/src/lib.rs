use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use thiserror::Error;
use uuid::Uuid;

pub const CASE_SCHEMA_VERSION: &str = "1.0.0";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Windows11X64,
    FedoraKde44X64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProblemCategory {
    Performance,
    Network,
    Updates,
    Storage,
    DriversGraphics,
    Services,
    General,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Critical,
    Warning,
    Notice,
    Info,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    ConfirmedProblem,
    StrongIndicator,
    PossibleCause,
    InformationalObservation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    pub id: String,
    pub collector_id: String,
    pub kind: String,
    pub collected_at: DateTime<Utc>,
    pub data: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub rule_id: String,
    pub title: String,
    pub severity: Severity,
    pub confidence: Confidence,
    pub platform: Platform,
    pub category: ProblemCategory,
    pub explanation: String,
    pub evidence_refs: Vec<String>,
    pub likely_impact: String,
    pub recommended_next_steps: Vec<String>,
    pub documentation_links: Vec<String>,
    pub rule_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SystemProfile {
    pub os_name: String,
    pub os_version: String,
    pub architecture: String,
    pub desktop: Option<String>,
    pub hostname: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CollectorState {
    Complete,
    Partial,
    Skipped,
    Failed,
    TimedOut,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CollectorRecord {
    pub collector_id: String,
    pub status: CollectorState,
    pub message: Option<String>,
    pub duration_ms: u64,
    pub output_truncated: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScanStatus {
    Complete,
    Partial,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RedactionManifest {
    pub version: u32,
    pub categories: BTreeMap<String, u64>,
    pub transformed_paths: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Zip,
    Html,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportRecord {
    pub exported_at: DateTime<Utc>,
    pub format: ExportFormat,
    pub destination_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SupportCase {
    pub schema_version: String,
    pub case_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub app_version: String,
    pub platform: Platform,
    pub category: ProblemCategory,
    pub scan_status: ScanStatus,
    pub system_profile: SystemProfile,
    pub collection_manifest: Vec<CollectorRecord>,
    pub findings: Vec<Finding>,
    pub evidence: Vec<Evidence>,
    pub redaction_manifest: RedactionManifest,
    pub export_history: Vec<ExportRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleDefinition {
    pub id: String,
    pub version: u32,
    pub platforms: Vec<Platform>,
    pub categories: Vec<ProblemCategory>,
    pub title: String,
    pub severity: Severity,
    pub confidence: Confidence,
    pub explanation: String,
    pub collector_id: String,
    pub condition: RuleCondition,
    pub likely_impact: String,
    pub recommended_next_steps: Vec<String>,
    pub documentation_links: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleCondition {
    pub path: String,
    pub operator: RuleOperator,
    pub value: Value,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleOperator {
    Equals,
    NotEquals,
    GreaterThan,
    GreaterOrEqual,
    LessThan,
    Contains,
    Exists,
}

#[derive(Debug, Error)]
pub enum RuleError {
    #[error("invalid rule document: {0}")]
    InvalidDocument(#[from] serde_json::Error),
}

pub fn parse_rules(documents: &[&str]) -> Result<Vec<RuleDefinition>, RuleError> {
    documents
        .iter()
        .map(|document| serde_json::from_str(document).map_err(RuleError::from))
        .collect()
}

pub fn bundled_rules() -> Result<Vec<RuleDefinition>, RuleError> {
    parse_rules(&[
        include_str!("../../../rules/common/storage-critical.json"),
        include_str!("../../../rules/fedora/failed-services.json"),
        include_str!("../../../rules/fedora/network-disconnected.json"),
        include_str!("../../../rules/fedora/pending-updates.json"),
        include_str!("../../../rules/windows/stopped-auto-services.json"),
    ])
}

pub fn evaluate_rules(
    rules: &[RuleDefinition],
    evidence: &[Evidence],
    platform: Platform,
    category: ProblemCategory,
) -> Vec<Finding> {
    let mut findings: Vec<_> = rules
        .iter()
        .filter(|rule| rule.platforms.contains(&platform))
        .filter(|rule| rule.categories.contains(&category) || category == ProblemCategory::General)
        .flat_map(|rule| {
            evidence
                .iter()
                .filter(move |item| item.collector_id == rule.collector_id)
                .filter(move |item| condition_matches(&item.data, &rule.condition))
                .map(move |item| Finding {
                    rule_id: rule.id.clone(),
                    title: rule.title.clone(),
                    severity: rule.severity,
                    confidence: rule.confidence,
                    platform,
                    category,
                    explanation: rule.explanation.clone(),
                    evidence_refs: vec![item.id.clone()],
                    likely_impact: rule.likely_impact.clone(),
                    recommended_next_steps: rule.recommended_next_steps.clone(),
                    documentation_links: rule.documentation_links.clone(),
                    rule_version: rule.version,
                })
        })
        .collect();
    findings.sort_by(|left, right| {
        severity_rank(left.severity)
            .cmp(&severity_rank(right.severity))
            .then_with(|| left.rule_id.cmp(&right.rule_id))
    });
    findings
}

fn severity_rank(severity: Severity) -> u8 {
    match severity {
        Severity::Critical => 0,
        Severity::Warning => 1,
        Severity::Notice => 2,
        Severity::Info => 3,
    }
}

fn condition_matches(data: &Value, condition: &RuleCondition) -> bool {
    let current = condition
        .path
        .split('.')
        .try_fold(data, |value, segment| value.get(segment));
    match condition.operator {
        RuleOperator::Exists => current.is_some(),
        RuleOperator::Equals => current == Some(&condition.value),
        RuleOperator::NotEquals => current != Some(&condition.value),
        RuleOperator::Contains => current.is_some_and(|value| match value {
            Value::String(text) => condition.value.as_str().is_some_and(|needle| text.contains(needle)),
            Value::Array(values) => values.contains(&condition.value),
            _ => false,
        }),
        RuleOperator::GreaterThan => compare_numbers(current, &condition.value) == Some(Ordering::Greater),
        RuleOperator::GreaterOrEqual => matches!(
            compare_numbers(current, &condition.value),
            Some(Ordering::Greater | Ordering::Equal)
        ),
        RuleOperator::LessThan => compare_numbers(current, &condition.value) == Some(Ordering::Less),
    }
}

fn compare_numbers(left: Option<&Value>, right: &Value) -> Option<Ordering> {
    left?.as_f64()?.partial_cmp(&right.as_f64()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<Evidence> {
        serde_json::from_str(include_str!("../../../fixtures/fedora-general-evidence.json")).unwrap()
    }

    #[test]
    fn fixture_general_scan_is_deterministic() {
        let rules = bundled_rules().unwrap();
        let first = evaluate_rules(
            &rules,
            &fixture(),
            Platform::FedoraKde44X64,
            ProblemCategory::General,
        );
        let second = evaluate_rules(
            &rules,
            &fixture(),
            Platform::FedoraKde44X64,
            ProblemCategory::General,
        );
        assert_eq!(first, second);
        assert_eq!(first.len(), 4);
        assert_eq!(first[0].rule_id, "common.storage.root-critical");
    }

    #[test]
    fn category_filters_irrelevant_rules() {
        let findings = evaluate_rules(
            &bundled_rules().unwrap(),
            &fixture(),
            Platform::FedoraKde44X64,
            ProblemCategory::Network,
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "fedora.network.disconnected");
    }

    #[test]
    fn missing_paths_fail_closed() {
        let condition = RuleCondition {
            path: "missing.value".into(),
            operator: RuleOperator::GreaterThan,
            value: Value::from(0),
        };
        assert!(!condition_matches(&Value::Object(Default::default()), &condition));
    }
}
