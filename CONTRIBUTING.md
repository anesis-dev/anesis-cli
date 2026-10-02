# Contributing to anesis-cli

Thanks for taking the time. This repository is the CLI; the registry API, the web
app, and the registry content live in separate repositories under
[`anesis-dev`](https://github.com/anesis-dev).

## Before you start

- **Bug fixes** — open a PR directly, with a test that fails without the fix.
- **New commands or flags** — open an issue first. Command names, flag names and
  `--json` output are a public contract as of 1.0.
- **New templates or addons** — those belong in
  [`templates`](https://github.com/anesis-dev/templates) and
  [`addons`](https://github.com/anesis-dev/addons), not here.
- **Security problems** — do not open an issue. See [SECURITY.md](SECURITY.md).

## Getting set up

You need Rust `1.88` or newer (edition 2024 plus let-chains).

```bash
cargo build
cargo run -- --help
```

To point the CLI at a local server instead of production:

```bash
export ANESIS_BACKEND_URL=http://localhost:4000
export ANESIS_FRONTEND_URL=http://localhost:3000
```

`ANESIS_DEBUG=1` swaps the friendly error messages for full error chains and
panic backtraces.

Enable the repository's git hooks once per clone — `core.hooksPath` is local
config, so it does not come with the checkout:

```bash
git config core.hooksPath .githooks
```

`.githooks/pre-push` then runs fmt, clippy and the test suite before every
push. `git push --no-verify` or `ANESIS_SKIP_HOOKS=1` bypasses it.

## Before opening a PR

All four must pass. CI runs the same commands on Linux, macOS and Windows,
using `cargo nextest run` — use it locally too so mutation-testing baselines
stay meaningful.

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo nextest run
cargo publish --dry-run --locked
```

## House rules

- **Two-space indentation**, enforced by `rustfmt.toml`. Run `cargo fmt`.
- **Errors go through `anyhow`**, and anything a user can hit should be an
  `AnesisError` variant so it gets a hint and a meaningful exit code. See
  `src/utils/errors.rs`.
- **`--json` output is snapshot-tested** in `tests/json_output.rs`. Changing a
  key is a breaking change; if you intend it, update the snapshot in the same PR
  and say so in the description.
- **Anything platform-specific needs a `#[cfg]` and a reason.** `sh -c` does not
  exist on Windows, `CreateProcess` ignores `PATHEXT`, and `diff` may be absent —
  each of those has already bitten this project once.
- **Interactive prompts need a non-interactive path.** Every prompt must be
  reachable via `--yes` and `--input NAME=VALUE`, and must fail with a clear
  message rather than a raw terminal error when there is no TTY.
- **New shell execution needs a gate.** Addon `run` steps are the only place the
  CLI executes arbitrary remote commands, and they require `--allow-run` outside
  an interactive session. Do not add a second such path without the same
  treatment — see [SECURITY.md](SECURITY.md).

## Adding a command

1. Add the variant to `src/cli/commands.rs` with an `about` and, if it has a
   short form, a `visible_alias` (not `alias` — the latter is invisible in
   `--help`).
2. Wire it in `src/main.rs`.
3. If it takes `--json`, add it to the `json_mode` match in `run()` so the
   upgrade notice does not corrupt machine-readable output, and add a snapshot
   test.
4. If an AI agent should be able to drive it, add it to `src/mcp.rs` and its
   `tools_list()` schema.

## Commit messages and PRs

Conventional-commit prefixes (`feat:`, `fix:`, `refactor:`, `docs:`, `test:`,
`chore:`) are preferred but not enforced. Say what changed and why; if it changes
the CLI surface, note whether `CHANGELOG.md` needs an entry.

## Releasing

Releases are cut by tagging `vX.Y.Z`. That triggers `release.yml`, which builds
for five targets, generates `SHA256SUMS`, publishes the GitHub release, and then
pushes to crates.io and npm. `smoke.yml` also runs on the tag.

## Manual install-script checklist

`install.sh`, `install.ps1`, and `npm/install.js` have no automated test
harness — verify these by hand before a release that touches them:

- `npm/install.js` behind a proxy: set `HTTPS_PROXY` (or `HTTP_PROXY`,
  `npm_config_https_proxy`) to a local proxy (e.g. `mitmproxy`) and confirm the
  binary still downloads through it.
- To skip the binary download entirely (e.g. to drop in a locally-built
  binary), set `ANESIS_SKIP_INSTALL=1` before `npm install`, then place the
  binary at `npm/bin/anesis` (`npm/bin/anesis.exe` on Windows) yourself.
- `install.ps1` PATH handling: run it twice in a row on a clean Windows user
  profile and confirm the second run reports "already in your PATH" instead
  of appending a duplicate entry, and that an existing `REG_EXPAND_SZ` `Path`
  (e.g. containing `%JAVA_HOME%\bin`) is not collapsed to a literal path.
