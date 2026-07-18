use regex::{Captures, Regex};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;
use supportkit_core::RedactionManifest;

static EMAIL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b").expect("valid email regex")
});
static IPV4: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b").expect("valid IPv4 regex")
});
static IPV6: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:[0-9a-f]{1,4}:){2,7}[0-9a-f]{0,4}\b").expect("valid IPv6 regex")
});
static MAC: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:[0-9a-f]{2}[:-]){5}[0-9a-f]{2}\b").expect("valid MAC regex")
});
static LINUX_HOME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"/(?:home|var/home)/[^/\s\"']+").expect("valid Linux home regex")
});
static WINDOWS_HOME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b[A-Z]:\\Users\\[^\\\s\"']+").expect("valid Windows home regex")
});
static BEARER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bbearer\s+[A-Za-z0-9._~+/-]{8,}=*").expect("valid bearer regex")
});
static API_TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:sk|ghp|github_pat|xox[baprs])[-_][A-Za-z0-9_-]{12,}\b")
        .expect("valid token regex")
});

#[derive(Debug, Clone, Copy)]
pub struct RedactionOptions {
    pub redact_ssids: bool,
}

impl Default for RedactionOptions {
    fn default() -> Self {
        Self { redact_ssids: true }
    }
}

#[derive(Debug, Clone)]
pub struct RedactionResult {
    pub value: Value,
    pub manifest: RedactionManifest,
}

pub fn redact(value: &Value, options: RedactionOptions) -> RedactionResult {
    let mut copy = value.clone();
    let mut context = Context::default();
    visit(&mut copy, "$", None, options, &mut context);
    RedactionResult {
        value: copy,
        manifest: RedactionManifest {
            version: 1,
            categories: context.categories,
            transformed_paths: context.paths.into_iter().collect(),
            warnings: vec![
                "Review unclassified free-text evidence before sharing this case.".into(),
            ],
        },
    }
}

#[derive(Default)]
struct Context {
    categories: BTreeMap<String, u64>,
    paths: BTreeSet<String>,
}

impl Context {
    fn record(&mut self, category: &str, path: &str, count: u64) {
        if count == 0 {
            return;
        }
        *self.categories.entry(category.to_owned()).or_default() += count;
        self.paths.insert(path.to_owned());
    }
}

fn visit(
    value: &mut Value,
    path: &str,
    key: Option<&str>,
    options: RedactionOptions,
    context: &mut Context,
) {
    if let Some(category) = key.and_then(|name| classify_sensitive_key(name, options)) {
        if !value.is_null() {
            *value = Value::String(format!("[REDACTED:{category}]"));
            context.record(category, path, 1);
        }
        return;
    }

    match value {
        Value::Object(map) => {
            for (name, child) in map {
                visit(
                    child,
                    &format!("{path}.{}", escape_path(name)),
                    Some(name),
                    options,
                    context,
                );
            }
        }
        Value::Array(values) => {
            for (index, child) in values.iter_mut().enumerate() {
                visit(child, &format!("{path}[{index}]"), key, options, context);
            }
        }
        Value::String(text) => redact_text(text, path, context),
        _ => {}
    }
}

fn classify_sensitive_key(key: &str, options: RedactionOptions) -> Option<&'static str> {
    let normalized = key.to_ascii_lowercase().replace(['-', ' '], "_");
    if normalized == "hostname" || normalized.ends_with("_hostname") {
        return Some("hostname");
    }
    if matches!(normalized.as_str(), "username" | "user_name" | "account_name") {
        return Some("username");
    }
    if normalized == "ssid" && options.redact_ssids {
        return Some("wifi_ssid");
    }
    if normalized.contains("serial") || normalized == "machine_id" || normalized == "product_uuid" {
        return Some("machine_identifier");
    }
    if ["token", "api_key", "apikey", "password", "passwd", "secret", "authorization", "cookie"]
        .iter()
        .any(|needle| normalized == *needle || normalized.ends_with(&format!("_{needle}")))
    {
        return Some("credential");
    }
    None
}

fn redact_text(text: &mut String, path: &str, context: &mut Context) {
    replace_regex(text, &EMAIL, "email", path, context, |_| true);
    replace_regex(text, &MAC, "mac_address", path, context, |_| true);
    replace_regex(text, &IPV4, "ip_address", path, context, |matched| {
        matched.parse::<std::net::Ipv4Addr>().is_ok()
    });
    replace_regex(text, &IPV6, "ip_address", path, context, |matched| {
        matched.parse::<std::net::Ipv6Addr>().is_ok()
    });
    replace_regex(text, &LINUX_HOME, "home_path", path, context, |_| true);
    replace_regex(text, &WINDOWS_HOME, "home_path", path, context, |_| true);
    replace_regex(text, &BEARER, "credential", path, context, |_| true);
    replace_regex(text, &API_TOKEN, "credential", path, context, |_| true);
}

fn replace_regex<F>(
    text: &mut String,
    pattern: &Regex,
    category: &str,
    path: &str,
    context: &mut Context,
    valid: F,
) where
    F: Fn(&str) -> bool,
{
    let mut count = 0_u64;
    let replacement = pattern.replace_all(text, |captures: &Captures<'_>| {
        let matched = captures.get(0).map_or("", |value| value.as_str());
        if valid(matched) {
            count += 1;
            format!("[REDACTED:{category}]")
        } else {
            matched.to_owned()
        }
    });
    if count > 0 {
        *text = replacement.into_owned();
        context.record(category, path, count);
    }
}

fn escape_path(value: &str) -> String {
    value.replace('~', "~0").replace('.', "~1")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn adversarial_identifiers_and_secrets_are_removed() {
        let source = json!({
            "hostname": "alice-workstation",
            "username": "alice",
            "serialNumber": "SN-REAL-12345",
            "ssid": "Alice Home",
            "message": "alice@example.test from 192.168.1.24 fe80::1 at aa:bb:cc:dd:ee:ff in /home/alice/private and C:\\Users\\Alice\\Secret using Bearer abcdefghijklmnop",
            "nested": { "apiKey": "sk-secretsecretsecret", "safe": "Fedora 44" }
        });
        let result = redact(&source, RedactionOptions::default());
        let serialized = serde_json::to_string(&result.value).unwrap();
        for forbidden in [
            "alice-workstation", "alice@example.test", "192.168.1.24", "fe80::1",
            "aa:bb:cc:dd:ee:ff", "/home/alice", "Users\\\\Alice", "abcdefghijklmnop",
            "sk-secretsecretsecret", "SN-REAL-12345", "Alice Home",
        ] {
            assert!(!serialized.contains(forbidden), "leaked {forbidden}");
        }
        assert!(result.manifest.categories["credential"] >= 2);
        assert!(!result.manifest.transformed_paths.is_empty());
        assert!(!serde_json::to_string(&result.manifest).unwrap().contains("alice"));
    }

    #[test]
    fn invalid_ipv4_like_versions_are_not_redacted() {
        let result = redact(&json!({"version": "999.999.999.999"}), RedactionOptions::default());
        assert_eq!(result.value["version"], "999.999.999.999");
    }

    #[test]
    fn ssid_redaction_can_be_disabled_explicitly() {
        let result = redact(
            &json!({"ssid": "Synthetic Network"}),
            RedactionOptions { redact_ssids: false },
        );
        assert_eq!(result.value["ssid"], "Synthetic Network");
    }
}

