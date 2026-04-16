# Oxide

Oxide is a Rust CLI for scaffolding JavaScript and TypeScript projects from remote Oxide templates and extending them with project addons.

It supports:

- creating a new project from a template
- checking cached templates and addons for updates before use
- authenticating against the Oxide service
- publishing GitHub repositories as Oxide templates
- installing cached addons and running addon commands inside a project
- upgrading the CLI from GitHub Releases with `oxide upgrade`
- notifying you when a newer Oxide version is available

## Official website

The official Oxide website is [oxide-cli.vercel.app](https://oxide-cli.vercel.app/).

Use it as the main guide for learning and using Oxide:

- [Docs](https://oxide-cli.vercel.app/docs) cover installation, authentication, templates, addons, publishing, and reference material.
- [Templates](https://oxide-cli.vercel.app/templates) lists available project starters.
- [Addons](https://oxide-cli.vercel.app/addons) lists reusable workflow addons.

This README keeps a quick CLI reference for the repository and package users. The website contains the full, up-to-date usage guide.

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
npm install -g @oxide-cli/oxide
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

Oxide checks whether the cached template is current before generating the project. If a newer template version exists, it updates the local cache first.

## Command overview

Top-level commands:

```text
oxide new <NAME> <TEMPLATE_NAME>
oxide template <COMMAND>
oxide login
oxide logout
oxide account
oxide addon <COMMAND>
oxide upgrade
oxide completions <SHELL>
oxide use <ADDON_ID> <COMMAND>
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
oxide use <ADDON_ID> <COMMAND>
```

Example:

```bash
oxide addon install drizzle
cd my-app
oxide use drizzle init
```

Upgrade Oxide itself:

```bash
oxide upgrade
```

Install shell completions:

```bash
oxide completions zsh
```

Supported shells are `bash`, `zsh`, `fish`, and `powershell`.

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

When you run `oxide new`, Oxide also refreshes the template cache automatically if a newer version is available.

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

Run an installed addon command:

```bash
oxide use drizzle init
```

When you run an addon command such as `oxide use drizzle init`, Oxide checks for a newer cached addon version first. If the add-on updated and the command is marked `once: true`, Oxide prompts you to re-run it.

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

Upgrade the CLI to the latest release:

```bash
oxide upgrade
```

After most commands, Oxide performs a background version check and prints a short upgrade notice when a newer CLI release is available.

Install shell completions:

```bash
oxide completions zsh
```

Supported shells are `bash`, `zsh`, `fish`, and `powershell`.

## Local data and generated files

Oxide stores local state under `~/.oxide/`:

- cached templates in `~/.oxide/cache/templates`
- cached addons in `~/.oxide/cache/addons`
- template cache index in `~/.oxide/cache/templates/oxide-templates.json`
- addon cache index in `~/.oxide/cache/addons/oxide-addons.json`
- CLI version-check cache in `~/.oxide/version_check.json`
- authentication data in `~/.oxide/auth.json`

When addon commands run inside a project, Oxide records execution state in `oxide.lock` in the project root, including the add-on version used for each executed command.

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

## License

Licensed under either of these:

- MIT license ([LICENSE-MIT](LICENSE-MIT))
- Apache License 2.0 ([LICENSE-Apache](LICENSE-Apache))
