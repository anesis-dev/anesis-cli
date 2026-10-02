# Anesis

Anesis is a Rust CLI for scaffolding projects from remote templates and extending them
with reusable, versioned project addons.

## Install

```bash
curl -sSL https://raw.githubusercontent.com/anesis-dev/anesis-cli/main/install.sh | bash   # Linux/macOS
irm https://raw.githubusercontent.com/anesis-dev/anesis-cli/main/install.ps1 | iex          # Windows
npm install -g anesis-cli                                                              # npm
cargo install anesis                                                                   # cargo
```

## Quick start

```bash
anesis login                        # authenticate (required for remote templates/addons)
anesis new my-app nest-express       # scaffold a project from a template
cd my-app
anesis addon install nest-prisma-v7
anesis use nest-prisma-v7 generate   # apply an addon command to the project
anesis status                       # show the project's template + applied addons
```

## Commands

```text
anesis new <NAME> [TEMPLATE]        create a project (--stack to scaffold template + addons; --dry-run to preview)
anesis template <install|link|list|info|remove|publish|republish>
anesis addon <install|link|list|info|test|remove|publish|republish>
anesis stack <install|link|list|info|remove|publish|republish>
anesis use [ADDON] [COMMAND]        run an addon command in the current project (--dry-run to preview, --diff to try it on a scratch copy)
anesis undo <ADDON>                 revert an applied addon's changes
anesis outdated / anesis update <ADDON>|--all
anesis why [PATH]                   show which addon command created/modified a file (omit PATH to list all)
anesis doctor                       diagnose common environment/project problems
anesis search [QUERY]               search templates/addons/stacks
anesis login / logout / account
anesis mcp                          run an MCP stdio server for AI agents
anesis completions <shell>          bash, zsh, fish, powershell
anesis info / anesis status
```

Short aliases: `n` new, `t` template, `a` addon, `s` stack, `in` login, `out` logout.
Most commands accept `--json` for machine-readable output and `--yes` to skip prompts.

### The three update verbs

They are easy to confuse, so they are named apart:

| Command | Updates |
|---|---|
| `anesis update <ADDON>` | an addon **already applied to this project**, to the registry's latest version |
| `anesis upgrade` (alias `self-update`) | **the anesis binary itself** |
| `anesis <template\|addon\|stack> republish <URL>` | a **registry entry**, re-read from its GitHub repository |

`republish` was called `update` before 1.0.0; the old spelling still works as a
hidden alias, so existing scripts keep running.

### Global flags

Available on every subcommand:

| Flag | Effect |
|---|---|
| `-v`, `--verbose` | debug logging for anesis itself (`-vv` for trace). Goes to stderr, so `--json` stdout stays clean. |
| `-q`, `--quiet` | no progress spinners, no upgrade notice. Results, errors and exit codes are unchanged. |
| `--no-color` | plain output. Also implied when stdout is not a terminal, or when `NO_COLOR` is set. |
| `--ascii` | ASCII glyphs (`[ok]`, `->`, `*`) instead of Unicode (`✓`, `→`, `•`). Also implied by `ANESIS_ASCII`, non-UTF-8 locales, and plain `cmd.exe`. |
| `--no-telemetry` | see [Telemetry](#telemetry). |
| `--allow-run` | see [Running remote code](#running-remote-code). |

Full command reference: https://anesis-dev.vercel.app/docs/reference/commands. Docs and
template/addon catalog: https://anesis-dev.vercel.app.

## Man pages

Release archives ship `man/`, and the binary can regenerate the pages itself:

```sh
anesis man ./man        # writes anesis.1 plus one page per subcommand
```

This exists for packagers (Homebrew, Scoop, distro packages); see
[`packaging/README.md`](./packaging/README.md).

## Compatibility

Templates declare `anesisVersion` and addons/stacks declare `schema_version`.
Since 1.0.0 both are **enforced**: a manifest that needs a newer CLI is refused
with an upgrade hint rather than applied best-effort. Full policy in
[`COMPATIBILITY.md`](./COMPATIBILITY.md).

## Local data

Anesis stores state under `~/.anesis/`: cached templates/addons, `auth.json`, and a
version-check cache. Inside a scaffolded project, applied addons are tracked in
`anesis.lock`.

## Environment variables

| Variable | Default | What it does |
|---|---|---|
| `ANESIS_BACKEND_URL` | `https://api.anesis.tech` | Registry API to talk to. |
| `ANESIS_FRONTEND_URL` | `https://anesis.tech` | Web app used for login redirects. |
| `ANESIS_TOKEN` | — | Personal access token, for CI. Skips `anesis login`. |
| `ANESIS_NO_TELEMETRY` | unset | Set to disable install-count reporting (see below). |
| `ANESIS_ALLOW_RUN` | unset | Set to permit addon `run`/`packages` steps without a prompt (see below). |
| `ANESIS_DEBUG` | unset | Full error chains and panic backtraces instead of friendly messages. |
| `ANESIS_RELEASES_API_URL` | GitHub releases API | Override for `anesis upgrade`; mainly for testing. |
| `ANESIS_RELEASES_DOWNLOAD_BASE_URL` | GitHub releases downloads | Override for `anesis upgrade`; mainly for testing. |
| `RUST_LOG` | unset | Standard `env_logger` filter, e.g. `RUST_LOG=debug`. Overrides `-v`. |
| `NO_COLOR` | unset | Standard [no-color](https://no-color.org/) opt-out; same effect as `--no-color`. |
| `ANESIS_ASCII` | unset | Same effect as `--ascii`. |

## Telemetry

`anesis new` and every addon command send **one** request to the registry:
`POST /{template,addon,stack}/{id}/use`. That is what produces the download and
install counts shown on the website and on the `anesis` badges.

What it contains: your account token, the resource name, and a server-side
timestamp. What it does not contain: project contents, file paths, directory
names, input values, or anything about your machine.

It is skipped entirely when you are not logged in, and you can turn it off while
staying logged in:

```bash
anesis new my-app some-template --no-telemetry
export ANESIS_NO_TELEMETRY=1        # or set it once for the whole shell
```

## Running remote code

Addons can contain `run` steps (arbitrary shell commands) and `packages` steps
(package manager installs, which run the installed packages' own lifecycle
scripts) — both come from a registry entry, not from you. Anesis asks before
each one.

`--yes` does **not** cover this. Skipping the prompt requires `--allow-run` (or
`ANESIS_ALLOW_RUN=1`) so that "accept the defaults" and "execute code someone
else wrote" stay separate decisions. This matters most for `--stack` and
for `anesis mcp`, where an AI agent drives the CLI and there is nobody to ask.

The full trust model is in [SECURITY.md](SECURITY.md).

## License

[PolyForm Noncommercial License 1.0.0](LICENSE.md) — source available, noncommercial
use only.
