# Kubidm Repository Copilot Instructions (Lean)

Optimized high-signal guidance for this Rust monorepo (Identity management server + CLI + Python SDK + Unix integration). Always prefer existing Make targets.

## Project Overview

- Purpose: Simple, secure, fast identity management platform (LDAP, OAuth2/OIDC, RADIUS, Unix PAM/NSS).
- Stack: Rust stable, Cargo workspace, Python (pykubidm), JavaScript (Web UI), Docker multi-arch, mdBook docs.
- Components: `server/daemon`, `server/core`, `server/lib`, `libs/client`, `tools/cli`, `unix_integration/`, `pykubidm/`.
- Registry: `ghcr.io/pando85/kubidm/server`, `ghcr.io/pando85/kubidm/tools`, `ghcr.io/pando85/kubidm/radius`.

## Engineering Contract

- Smallest correct diff; no speculative refactors unless requested.
- Search & confirm target module before editing; keep crate boundaries clean.
- All clippy warnings must be zero (`make precommit`).
- **Disallowed**: `std::collections::HashMap/HashSet` (use BTreeMap/BTreeSet), `time::OffsetDateTime::now_utc` (inject time).
- Async: never block Tokio; use proper async patterns.
- Errors/logging: proper error types, `tracing` for logging; avoid unwrap in production.
- Dependencies: add only if necessary; minimal additions.
- Performance: avoid unnecessary `clone`; prefer refs/iterators.
- Output: provide minimal diff hunks, not whole files.
- Imports: Always place `use` statements at top level, grouped (std, external, internal).
- Underspecified ask: state <=2 assumptions, proceed.

LLM Style:
- Lead with intent + next action; bullets > prose; no filler.
- Provide deltas only on iterative turns.

## Make Targets Cheat Sheet

Lint / Build / Test:

- `make precommit` - All checks (test + codespell + pykubidm + doc/format)
- `make test` - Cargo test
- `make codespell` - Spell check
- `make doc/format` - Markdown format check
- `make doc/format/fix` - Fix markdown formatting

Build / Run:

- `make run` - Dev server (insecure)
- `make build/kubidmd` - Local Docker image
- `make buildx/kubidmd` - Multi-arch Docker push
- `make release/kubidm` - CLI release binary
- `make release/kubidmd` - Daemon release binary

Python (pykubidm):

- `make test/pykubidm` - All Python tests (pytest + mypy + ruff)
- `make test/pykubidm/pytest` - pytest only
- `make test/pykubidm/mypy` - mypy strict check
- `make test/pykubidm/lint` - ruff lint

Docs:

- `make book` - Build mdBook
- `make doc` - Rust docs
- `make docs/pykubidm/serve` - Local pykubidm docs

Release:

- `make prep` - Check outdated + audit

## Common Pitfalls

- Using `HashMap`/`HashSet` -> Use `BTreeMap`/`BTreeSet` for determinism.
- Calling `OffsetDateTime::now_utc` -> Inject time as parameter.
- Blocking calls in async -> Use proper async patterns.
- Forgetting `make precommit` -> CI will fail.
- Missing spell check -> Run `make codespell`.

## Trust These Instructions

Default to these; search only when commands fail or new subsystems emerge. Keep diffs minimal, tests authoritative.
