# Implementation plan

## Phase 1: foundations

Define scope, threat model, architecture decisions, versioned JSON contracts, repository commands, CI shape, and synthetic fixtures. Gate: schemas compile and fixtures validate.

## Phase 2: trusted core

Implement typed evidence/finding/case contracts, deterministic fixture-backed rule evaluation, adversarial structured redaction, bounded collector runtime, offline report generation, deterministic ZIP export, and atomic local persistence. Gate: Rust workspace tests and Clippy pass.

## Phase 3: desktop vertical slice

Implement category selection, scan progress, findings, privacy review, export actions, local history, settings, error/empty/loading states, and a browser fixture adapter. Gate: TypeScript, component tests, production build, and Playwright smoke flow pass.

## Phase 4: delivery readiness

Add GitHub Actions, Dependabot, packaging configuration/scaffolding, user/developer/rule/format documentation, and validate the complete root command surface. Gate: `just check`, frontend build, and platform-feasible package checks.

## Milestone exit

The milestone is usable when a Fedora KDE user can launch the Tauri app, run a supported scan without elevation, review deterministic findings and redacted evidence, export a valid bundle, and reopen the persisted case. Windows code must compile in CI and use the same contracts, but installer and live-collector validation remain target-platform gates.
