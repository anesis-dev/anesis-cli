# Security Policy

## Supported versions

| Version | Supported |
|---|---|
| 1.0.x | ✅ |
| < 1.0 | ❌ |

Run `anesis upgrade` to move to the latest release. There is no back-porting to
older tags.

## Reporting a vulnerability

**Do not open a public issue for a security problem.**

Use GitHub's private vulnerability reporting on this repository:
[Security → Report a vulnerability](https://github.com/anesis-dev/anesis-cli/security/advisories/new).

Please include what the issue is, where in the code it lives, how to reproduce
it, and what an attacker gains. You should get an acknowledgement within 7 days.
Confirmed issues are fixed, released, and then published as a GitHub Security
Advisory crediting you unless you ask otherwise.

## The trust model

Anesis downloads and executes content authored by other people. This section
says exactly how much trust each piece gets, so you can decide what you are
comfortable with.

### Templates

A template is a directory of files rendered through MiniJinja and written into your
new project. Nothing in a template executes during scaffolding. Template paths
are normalised and refused if they would escape the output directory.

The generated project is, of course, ordinary source code — running
`npm install && npm run dev` in it runs the template author's dependencies and
scripts. That is no different from cloning any starter repository, and Anesis
does not do it for you.

### Addons

An addon is a manifest of steps applied to an existing project. Most step kinds
(`copy`, `create`, `inject`, `replace`, `append`, `delete`, `rename`, `move`)
only read and write files under the project root, and each records a rollback
entry so `anesis undo` can reverse it.

Two step kinds do more:

- **`packages`** invokes your package manager (`npm`, `bun`, `pnpm`, `yarn`) to
  install the dependencies the addon names. Installing a package runs that
  package's own lifecycle scripts.
- **`run`** executes an arbitrary shell command. It is the one step that cannot
  be rolled back — `anesis undo` reports it as an irreversible action and leaves
  its effects in place.

### `run` and `packages` steps require explicit consent

Both execute code you did not write: `run` is an arbitrary shell command, and
`packages` invokes your package manager, which runs the installed packages'
own lifecycle scripts. Interactively, Anesis prints the exact command and asks
before executing it. The default answer is **no**.

Non-interactively there is nobody to ask, so a `run` or `packages` step is
**refused** unless you pass `--allow-run` or set `ANESIS_ALLOW_RUN=1`:

```bash
anesis use some-addon setup --yes                # run/packages steps are refused
anesis use some-addon setup --yes --allow-run    # run/packages steps execute
```

`--yes` deliberately does not imply `--allow-run`. "Accept the defaults" and
"execute shell commands written by a stranger" are different decisions, and the
paths that used to conflate them are exactly the risky ones:

- `anesis new --stack ...` is always non-interactive for the addons it applies.
- `anesis mcp` hardcodes `--yes` on every mutating tool, so an AI agent driving
  the CLI would otherwise run unreviewed remote shell with no prompt anywhere.
  The MCP tools expose `allow_run` as an explicit boolean the agent must set.

### What to do before applying an unfamiliar addon

```bash
anesis addon info <addon-id>        # read the manifest, including every step
anesis use <addon-id> <command> --dry-run   # print the plan, change nothing
anesis addon test <addon-id> <command>      # apply to a throwaway copy, show a diff
```

### Credentials

`anesis login` stores a JWT in `~/.anesis/auth.json`, written with mode `0600` on
Unix. `ANESIS_TOKEN` overrides it and is the intended mechanism for CI.

A personal access token carries the **full rights of your account**, including
admin if you have it. Scoped tokens do not exist yet. Give CI tokens an expiry
(`anesis` account page → tokens → 30/90/365 days) rather than a permanent one.

During `anesis login` the CLI starts a listener on `127.0.0.1` (ports 8080–8089)
and the backend redirects the browser to it with the JWT in the query string.
That URL never leaves the loopback interface; it is a query parameter rather than
a fragment because the local listener cannot read a fragment.

## Verifying release artifacts

Every release publishes `SHA256SUMS` alongside the archives, and both the
archives and `SHA256SUMS` carry a
[GitHub build provenance attestation](https://docs.github.com/en/actions/security-guides/using-artifact-attestations-to-establish-provenance-for-builds)
proving they were built by this repository's release workflow from the tagged
commit — not hand-uploaded.

```bash
curl -LO https://github.com/anesis-dev/anesis-cli/releases/download/v1.0.0/anesis-linux-x86_64.tar.gz
curl -LO https://github.com/anesis-dev/anesis-cli/releases/download/v1.0.0/SHA256SUMS
sha256sum -c SHA256SUMS --ignore-missing

gh attestation verify anesis-linux-x86_64.tar.gz --repo anesis-dev/anesis-cli
gh attestation verify SHA256SUMS --repo anesis-dev/anesis-cli
```

## Out of scope

- Vulnerabilities in the dependencies a template or addon installs — report those
  to their upstream.
- A `run` step doing something harmful *after* you explicitly approved it. That is
  the documented behaviour of the feature; the registry's moderation, not the
  CLI, is the control there.
- Reports from automated scanners with no demonstrated impact.

## Related

Server-side issues (the registry API, authentication, publishing) belong in the
[anesis-server](https://github.com/anesis-dev/anesis-server/blob/main/SECURITY.md)
repository.
