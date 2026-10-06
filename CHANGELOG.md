# Changelog

All notable changes to the Anesis CLI are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Versions prior to 1.0.0 were not tracked in this file; see the
[GitHub releases](https://github.com/anesis-dev/anesis-cli/releases) for that history.

## [Unreleased]

## [1.0.1]

### Fixed

- `anesis use --diff` no longer prints gitignored files (such as `.env`) or `node_modules`; both sides of the
  diff are now built with the same gitignore-aware copy, and `.git` is excluded.
- `anesis update` re-applies addon-level inputs, asks for confirmation once before undoing anything, and
  restores the previous version (files, `anesis.lock`, `anesis.json`) if re-applying fails.
- Ctrl-C, SIGTERM and SIGHUP now stop the running command cooperatively: the rollback runs on the main thread
  after the current step finishes, never concurrently with it. Cancelling a prompt exits with 130 (interrupt)
  or 7 (cancel) instead of 1.
- Choosing "Keep changes" after a failed step records the partial journal in `anesis.lock` (command name
  suffixed with ` (incomplete)`) so `anesis undo` can still revert it. Rollback failures are reported instead
  of being swallowed, and leftovers are kept in the lock.
- A template render error removes the half-generated project, so the command can be retried.
- Executable permissions of template files are preserved in generated projects.
- `json_patch` keeps key order and indentation, supports JSON Pointer paths (`/exports/~1package.json`),
  and errors instead of silently replacing non-object parents.
- Project commands (`use`, `undo`, `update`, `outdated`, `status`, `doctor`, `why`) find the project root by
  walking up to the nearest `anesis.json` / `anesis.lock`.
- `run` step commands shell-quote every interpolated value; `packages` steps reject specs starting with `-`.
- Unknown `--input` names are rejected (with a "did you mean" hint); `select` and `boolean` values are
  validated and booleans are normalised.
- `anesis.lock` stores original file contents as base64 (legacy byte arrays still load); journal entries
  under `.git`/`.hg`/`.svn` are ignored and restored modes are masked to `0o777`.
- Cache indexes, `anesis.json`, the version cache and `auth.json` are written atomically; the addon and
  template indexes are updated under a file lock. `auth.json` is created `0600`, `~/.anesis` `0700`.
  `write_atomic` follows symlinks and keeps the target's permissions.
- Stack IDs and template names are validated before they are used as cache paths.
- Updating a cached addon extracts to a temporary sibling and swaps it in only on success.
- The new-version check waits at most 300 ms at exit, uses a 2 s client and caches failures for an hour.
- Warnings and notices go to stderr; colour is disabled unless stderr is also a terminal; spinners, the
  picker footer and step arrows honour `--ascii`.
- MCP: positional arguments are separated with `--`, child processes time out after 5 minutes, and
  `ANESIS_ALLOW_RUN` is not inherited unless the call passes `allow_run: true`.
- `ANESIS_DEBUG=0` no longer enables debug output; 4xx registry responses no longer map to the network exit code.
- Other: stricter GitHub URL validation, complete Windows reserved names, `show_diff` reports `diff` failures,
  `outdated` queries addons concurrently, `update --all` continues past failures, `search` treats catalog
  failures consistently, the installer for zsh completions no longer calls `compinit` twice and bash
  completions honour `XDG_DATA_HOME`, `upgrade` refuses package-managed installs and cleans up its temp file.

### Changed

- `--json` objects built with `serde_json::json!` now keep insertion order instead of alphabetical order
  (`serde_json` `preserve_order`). The set of keys is unchanged.
- MSRV is now 1.89 (advisory file locking).

- Template rendering now uses MiniJinja instead of Tera. The `.tera` file suffix is unchanged. Filter
  arguments follow Jinja2 syntax: `replace("a", "b")`, `default("x")`, `join(",")`, `split(" ")`
  instead of Tera's `replace(from=..., to=...)`, `default(value=...)`, `join(sep=...)`, `split(pat=...)`.
  `slugify`, `now()`, `truncate` and the `containing`/`starting_with` tests are not available.
- A variable that is not provided still fails the render when printed, and is false inside `{% if %}`.

## [1.0.1]

### Fixed

- CI: allow the `unused-mut` lint only on Windows in the `windows-latest` clippy job.

## [1.0.0]

Anesis's first stable release. The CLI, registry manifest format, and
templates/addons/stacks it scaffolds are now covered by a compatibility policy
(see [`COMPATIBILITY.md`](./COMPATIBILITY.md)).

### Added

- `anesis new` — scaffold a project from a remote template, optionally combined
  with a stack (`--stack`) that installs a template plus a curated set of addons.
- `anesis template` / `anesis addon` / `anesis stack` — install, link, list,
  inspect, remove, publish, and republish registry entries.
- Stacks are multi-versioned and carry a structured `author` (`name` + `github`),
  mirroring templates and addons: publishing a new version adds a row, listings
  show the latest version with a version count, and re-publishing an existing
  version is rejected.
- `anesis use` / `anesis undo` — run and revert addon commands against a
  scaffolded project.
- `anesis outdated` / `anesis update` — check for and apply addon updates.
- `anesis search` — search templates, addons, and stacks.
- `anesis login` / `logout` / `account` — GitHub-backed authentication.
- `anesis mcp` — run an MCP stdio server exposing Anesis to AI agents.
- `anesis completions` — shell completions for bash, zsh, fish, and PowerShell.
- `anesis info` / `anesis status` — inspect CLI and project state.
- `anesis stack link` — validate a local `anesis.stack.json` and cache it, so a
  stack can be scaffolded with `anesis new <dir> --stack <id>` before it is
  published. Templates and addons already had this.
- `anesis outdated --json` — machine-readable version status per applied addon,
  distinguishing "up to date" from "could not reach the registry".
- Global `-v`/`--verbose`, `-q`/`--quiet` and `--no-color`, available on every
  subcommand. Diagnostics go to stderr, so `--json` stdout stays a single clean
  document. `NO_COLOR` is honoured.
- `anesis completions <shell> --print` — writes the script to stdout instead of
  installing it, for packagers whose builds must not touch the building user's
  dotfiles.
- Man pages: `anesis man <DIR>` writes `anesis.1` plus one page per subcommand.
  Release archives ship `anesis-man.tar.gz`; the pages are generated from the
  command definitions, so they cannot go stale.
- Homebrew tap and Scoop bucket manifests under `packaging/`, rendered from the
  release's `SHA256SUMS`. Inert until the tap repositories exist — see
  `packaging/README.md`.
- npm packages are now published with `--provenance` (Sigstore attestation
  binding the tarball to the release workflow run).

### Changed

- **`template|addon|stack update <URL>` is now `republish <URL>`.** This resolves
  a three-way collision with `anesis update <ADDON>` (upgrades an addon applied
  to a project) and `anesis upgrade` (replaces the CLI binary). `update` remains
  as a hidden alias, so existing scripts keep working, and `anesis upgrade`
  gained the alias `self-update`.
- **The crate now declares a machine-readable license.** `Cargo.toml` carried
  `license-file = "LICENSE.md"`, which tooling can only fuzzy-match; it is now
  `license = "PolyForm-Noncommercial-1.0.0"`. The license itself is unchanged —
  only its metadata — but downstream `cargo deny`/`cargo license` runs will start
  seeing it, and it is not an OSI-approved license. Registry contents
  (templates, addons, stacks) are Apache-2.0 and unaffected, so what you scaffold
  is not covered by the CLI's license.
- **`anesisVersion` and `schema_version` are now enforced.** Both were parsed and
  ignored, so a manifest written for a newer CLI was applied best-effort and
  could silently drop fields it did not understand. A manifest that needs a newer
  CLI is now refused with an upgrade hint. A prerelease build compares as its
  release version, and an unparseable `anesisVersion` warns rather than fails.
  See [`COMPATIBILITY.md`](./COMPATIBILITY.md).

### Fixed

- **`if_exists: "ask"` no longer makes an addon unusable non-interactively.** The
  prompt fired unconditionally, so any addon using it aborted with "the input
  device is not a TTY" under `--yes` — which covers every `--stack` apply and
  every `anesis mcp` call. It now resolves to the prompt's own default and keeps
  the user's existing file. Applies to `copy` and `create` steps alike.
- **A stack id that matched a directory name no longer shadows the stack.**
  `resolve_stack` treated any existing path as a local stack, so
  `anesis new app --stack rust-api` run beside an unrelated `rust-api/` folder
  failed with "could not read stack manifest" instead of using the published
  stack. A directory now only counts as a local stack when it holds an
  `anesis.stack.json`.
- `registry-lint` reported false errors for multi-variant addons: it checked
  every variant's `inject` anchors against the single `test-fixture/`, so a
  Tailwind addon was flagged for not finding `farm.config.ts` in a Vite fixture.
  It now evaluates the addon's own `detect` rules against the fixture and checks
  only the variant that would actually be applied — including `toml_contains`
  rules, which the Rust addons select on. It also resolves `glob` inject targets
  (previously skipped entirely) and enforces that a manifest `id` matches its
  directory name, which was documented but never checked.
- The `nest-saas` stack referenced three addon ids that did not exist or had
  been renamed (`nest-config`, `nest-prisma`, `nest-jwt-auth`); installs of the
  stack are now guaranteed to resolve.
- The README quick start referenced a nonexistent `drizzle` addon.
