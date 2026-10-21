# Board verification status

Source prepared on 2026-10-21. This is a record of incomplete verification, not a passing browser proof.

## Passed

- `cargo fmt --all --check`.
- `cargo metadata --offline --no-deps --format-version 1`: both workspace manifests are valid.
- `node --check tests/e2e/smoke.scenario.mjs`: browser scenario syntax is valid.
- `sh .offrails/tools/offrails-check`: no errors. Acceptance criteria are cited by tests, ownership/tenancy suites are present, IDs are 64-bit, environment variables are documented, and no unscoped controller lookups were found.
- `cargo offrails brand`: 16 contrast pairs passed WCAG AA. SVG assets generated.

## Blocked

- `cargo check --offline` and `CARGO_NET_OFFLINE=true cargo offrails verify --full` cannot resolve the template's required Askama version. Crates.io is unreachable from the environment, and essential dependencies are not cached. No Rust compilation, clippy, or application tests have passed.
- SQLite and Postgres migrations, rollback, acceptance and security tests await a successful build. Postgres is not configured here.
- Tailwind cannot be downloaded; CSS is not built. Browser screenshots and interaction checks are unavailable until the app runs.
- Chromium exited with SIGTRAP during brand rasterization. PNG assets still need regeneration.
- Coverage, cargo-deny and gitleaks tools are not installed.
- No baseline commit was possible. The workspace's pre-existing `.git` directory is protected and is not a usable repository.

## Resume verification

With network access, run `bin/setup`, `cargo offrails brand`, `cargo offrails verify`, and `UPDATE_SCHEMAS=1 cargo nextest run openapi`. Then start `bin/dev`, set `BASE` to its printed URL and `TEST_DATABASE_URL` to a scratch-capable Postgres connection, and run `cargo offrails verify --full`. Resolve any compiler or test findings before claiming the app is complete.

The browser smoke scenario is `tests/e2e/smoke.scenario.mjs`. Run the login smoke with that scenario, plus axe checks in light and dark. The framework's `prove` command replaces this file with screenshots and route/acceptance evidence after successful browser verification.
