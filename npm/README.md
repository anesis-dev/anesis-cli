# anesis-cli

npm distribution of **Anesis** — a Rust CLI for scaffolding projects from remote
templates and extending them with reusable, versioned project addons.

```bash
npm install -g anesis-cli
```

The `postinstall` script downloads the prebuilt binary for your platform from the
[GitHub release](https://github.com/anesis-dev/anesis-cli/releases) and verifies it
against the release `SHA256SUMS` before installing it.

Supported platforms: `linux-x64`, `linux-arm64`, `darwin-x64`, `darwin-arm64`,
`win32-x64`. On anything else, use one of the
[other install methods](https://github.com/anesis-dev/anesis-cli#install).

The download honors `HTTPS_PROXY`/`HTTP_PROXY` (and the npm-config equivalents)
if set. To skip the download and provide your own binary, set
`ANESIS_SKIP_INSTALL=1` before installing, then place it at `bin/anesis`
(`bin/anesis.exe` on Windows) inside this package's install directory.

## Quick start

```bash
anesis login                         # authenticate (required for remote templates/addons)
anesis new my-app nest-express       # scaffold a project from a template
cd my-app
anesis addon install nest-prisma-v7
anesis use nest-prisma-v7 generate   # apply an addon command to the project
anesis status                        # show the project's template + applied addons
```

## Documentation

This page only covers installing via npm. Everything else lives upstream:

- **Command reference** — https://anesis-dev.vercel.app/docs/reference/commands
- **Docs & template/addon catalog** — https://anesis-dev.vercel.app
- **Source, README, CHANGELOG** — https://github.com/anesis-dev/anesis-cli

## License

[PolyForm Noncommercial License 1.0.0](https://github.com/anesis-dev/anesis-cli/blob/main/LICENSE.md)
— source available, noncommercial use only.
