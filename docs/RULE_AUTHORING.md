# Rule authoring

Rules are versioned JSON documents under `rules/common`, `rules/fedora`, or `rules/windows`. They are deterministic mappings from normalized evidence to explanatory findings. Rules must not execute code, call a service, use AI, or mutate the host.

## Required fields

Use [rule.schema.json](../schemas/rule.schema.json) as the authority. Every rule defines a stable `id`, integer `version`, supported platforms/categories, severity, confidence, explanation, expected collector, one condition, likely impact, next steps, and documentation links.

Supported operators are `equals`, `not_equals`, `greater_than`, `greater_or_equal`, `less_than`, `contains`, and `exists`. Conditions address evidence with a dot path such as `data.usedPercent`. Missing paths fail closed and do not match.

## Confidence guidance

- `confirmed_problem`: evidence directly proves the described condition.
- `strong_indicator`: evidence is highly associated but does not prove the root cause.
- `possible_cause`: evidence is relevant but ambiguous.
- `informational_observation`: useful context without a problem claim.

Do not raise confidence to compensate for weak collection. Phrase limitations explicitly.

## Workflow

1. Add or update normalized synthetic evidence in `fixtures/`.
2. Add the smallest rule that matches that contract.
3. Increment `version` when behavior or text changes.
4. Add a deterministic core test for match and non-match cases.
5. Run `just generate-fixtures`, `npm run validate-schemas`, and `cargo test -p supportkit-core`.

Never put secrets, real hostnames, real addresses, or copied customer logs in fixtures.
