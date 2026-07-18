set shell := ["bash", "-uc"]

setup:
    npm install
    cargo fetch

dev:
    npm run tauri dev --workspace @loofi/supportkit-desktop

test:
    cargo test --workspace
    npm test

lint:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    npm run lint

typecheck:
    cargo check --workspace --all-targets
    npm run typecheck

check: lint typecheck test validate-schemas release-check
    npm run format:check

build:
    npm run build
    cargo build --workspace

validate-schemas:
    npm run validate-schemas

release-check:
    npm run validate:release

generate-fixtures:
    cargo test -p supportkit-core fixture_general_scan_is_deterministic

package:
    npm run tauri build --workspace @loofi/supportkit-desktop
