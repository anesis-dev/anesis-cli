# Oxide

Oxide is a Rust CLI for scaffolding JavaScript and TypeScript projects from remote Oxide templates and extending them with project addons.

It supports:

- creating a new project from a template
- caching templates locally and skipping unchanged downloads
- authenticating against the Oxide service
- publishing GitHub repositories as Oxide templates
- installing cached addons and running addon commands inside a project

## Installation

### Quick install

Linux and macOS:

```bash
curl -sSL https://raw.githubusercontent.com/oxide-cli/oxide/main/install.sh | bash
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/oxide-cli/oxide/main/install.ps1 | iex
```

Unix installers place the `oxide` binary in `~/.local/bin`. Cargo installs it in `~/.cargo/bin`. Make sure the relevant directory is in your `PATH`.

### Install with npm

```bash
npm install -g @oxide/oxide-cli
```

The npm package downloads the matching prebuilt Oxide binary during `postinstall`.

### Install with cargo

```bash
cargo install oxide-cli
```

### Manual install from GitHub Releases

Download the latest release artifact for your platform:

- [Linux x86_64](https://github.com/oxide-cli/oxide/releases/latest/download/oxide-linux-x86_64.tar.gz)
- [Linux ARM64](https://github.com/oxide-cli/oxide/releases/latest/download/oxide-linux-aarch64.tar.gz)
- [macOS Apple Silicon](https://github.com/oxide-cli/oxide/releases/latest/download/oxide-macos-aarch64.tar.gz)
- [Windows x86_64](https://github.com/oxide-cli/oxide/releases/latest/download/oxide-windows-x86_64.zip)

## Getting started

Most remote operations require authentication first:

```bash
oxide login
```

Create a new project from a template:

```bash
oxide new my-app react-vite-ts
```

If the template is not cached yet, Oxide downloads it automatically before generating the project.

## Command overview

Top-level commands:

```text
oxide new <NAME> <TEMPLATE_NAME>
oxide template <COMMAND>
oxide login
oxide logout
oxide account
oxide addon <COMMAND>
oxide <ADDON_ID> <COMMAND>
```

Template management:

```text
oxide template install <TEMPLATE_NAME>
oxide template list
oxide template remove <TEMPLATE_NAME>
oxide template publish <GITHUB_REPOSITORY_URL>
oxide template update <GITHUB_REPOSITORY_URL>
```

Addon management:

```text
oxide addon install <ADDON_ID>
oxide addon list
oxide addon remove <ADDON_ID>
oxide addon publish <GITHUB_REPOSITORY_URL>
oxide addon update <GITHUB_REPOSITORY_URL>
```

Addon execution:

```text
oxide <ADDON_ID> <COMMAND>
```

Example:

```bash
oxide addon install drizzle
cd my-app
oxide drizzle init
```

Aliases:

- `oxide n ...` for `oxide new ...`
- `oxide t ...` for `oxide template ...`
- `oxide a ...` for `oxide addon ...`
- `oxide in` for `oxide login`
- `oxide out` for `oxide logout`

## Common workflows

Install or refresh a template in the local cache:

```bash
oxide template install react-vite-ts
```

List cached templates:

```bash
oxide template list
```

Remove a cached template:

```bash
oxide template remove react-vite-ts
```

Show the authenticated account:

```bash
oxide account
```

Publish a GitHub repository as a template:

```bash
oxide template publish https://github.com/owner/repo
```

Update a published template:

```bash
oxide template update https://github.com/owner/repo
```

Install an addon:

```bash
oxide addon install drizzle
```

List installed addons:

```bash
oxide addon list
```

Remove a cached addon:

```bash
oxide addon remove drizzle
```

Publish a GitHub repository as an addon:

```bash
oxide addon publish https://github.com/owner/repo
```

Update a published addon:

```bash
oxide addon update https://github.com/owner/repo
```

## Local data and generated files

Oxide stores local state under `~/.oxide/`:

- cached templates in `~/.oxide/cache/templates`
- cached addons in `~/.oxide/cache/addons`
- template cache index in `~/.oxide/cache/templates/oxide-templates.json`
- addon cache index in `~/.oxide/cache/addons/oxide-addons.json`
- authentication data in `~/.oxide/auth.json`

When addon commands run inside a project, Oxide records execution state in `oxide.lock` in the project root.

## Templates

Published templates are expected to include an `oxide.template.json` manifest in the template root. Oxide uses that manifest to track template metadata such as template name, version, source repository, and whether the template is official.

Template files ending with `.tera` are rendered during project generation and written without the `.tera` suffix.

Available template variables:

- `project_name`
- `project_name_kebab`
- `project_name_snake`

## Addons

Installed addons are expected to include an `oxide.addon.json` manifest. Oxide uses addon manifests to define:

- user inputs
- project detection rules
- command variants
- file modification steps such as create, copy, inject, replace, append, delete, rename, and move

## Development

Run the CLI locally:

```bash
cargo run -- --help
```

Run tests:

```bash
cargo test
```

## License

Licensed under either of these:

- MIT license ([LICENSE-MIT](LICENSE-MIT))
- Apache License 2.0 ([LICENSE-Apache](LICENSE-Apache))
