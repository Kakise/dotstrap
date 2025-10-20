# dotstrap

`dotstrap` is a Rust-powered bootstrapper that turns a repository of templated
dotfiles into a reproducible macOS setup. It renders templates, decrypts
secrets sourced from your environment or filesystem, links the results into
your home directory, and installs the Homebrew packages declared alongside
those templates.

## Features

- **Template rendering:** Author configuration files with [Handlebars] syntax
  and render them with shared values plus secrets (`{{secrets.token}}`).
- **Secrets management:** Reference environment variables or external files
  without ever templating them directly.
- **Safe linking:** Files are rendered into `~/.dotstrap/generated` and then
  linked into the home directory with automatic backups for pre-existing files.
- **Homebrew automation:** Keep `brew` taps, formulae, and casks in version
  control and install them in one run.
- **First-class tests:** The crate ships with 100 % unit test coverage and CI
  enforcement.

## Install

### From source (requires Rust)

```bash
cargo install dotstrap --git https://github.com/Kakise/dotstrap.git
```

## Code Structure

- `src/application/` – orchestrates the end-to-end workflow and exposes `run`.
- `src/cli/` – CLI definition built with `clap::Parser`.
- `src/config/` – strongly typed manifest and Homebrew loaders.
- `src/infrastructure/` – integrations for commands, repositories, and secrets.
- `src/services/` – reusable operations such as rendering, linking, and brew installation.
- `src/errors.rs` – shared error enums returned by all layers.

[handlebars]: https://handlebarsjs.com/

## Repository layout

```ignore
manifest.yaml           # Template manifest (required)
values.yaml             # Shared key/value pairs (optional)
brew/packages.yaml      # Homebrew taps/formulae/casks (optional)
secrets/secrets.yaml    # Secret sources (optional)
templates/              # Handlebars templates referenced by the manifest
```

### `manifest.yaml`

```yaml
version: 1
templates:
  - source: templates/gitconfig.hbs
    destination: .gitconfig
    mode: 0o600            # optional (UNIX only)
```

### `secrets/secrets.yaml`

```yaml
github_token:
  from: env
  key: DOTSTRAP_GITHUB_TOKEN
signing_key:
  from: file
  path: secrets/signing.asc
```

Secrets are injected under a `secrets` namespace inside templates. The example
above exposes `{{secrets.github_token}}` and `{{secrets.signing_key}}`.

### `brew/packages.yaml`

```yaml
taps:
  - homebrew/cask-fonts
formulae:
  - ripgrep
  - neovim
casks:
  - iterm2
```

## CLI

```bash
dotstrap ~/src/dotstrap-config
dotstrap git@github.com:me/dotstrap-config.git --dry-run
dotstrap ./config --home /tmp/home --skip-brew
```

Positional arguments and flags:

- `SOURCE` – required configuration repository (path or git URL).
- `--home <path>` – override the home directory (useful in tests).
- `--skip-brew` – skip Homebrew operations.
- `--dry-run` – render and report without modifying the filesystem.

## Secrets workflow

1. Declare a secret in `secrets/secrets.yaml` as either an environment variable
   (`from: env`) or a file (`from: file`). Relative file paths resolve within
   the configuration repository.
2. Set the environment variable (or create the file) **before** running
   `dotstrap`.
3. Reference the secret with `{{secrets.NAME}}` inside any template.

Missing secrets abort the run with a clear error to prevent partially rendered
dotfiles.

## Development

```bash
cargo fmt
cargo test
# coverage (CI enforces 100 %)
cargo tarpaulin --fail-under 100
```

### GitHub Actions

`.github/workflows/ci.yml` runs `cargo fmt`, `cargo clippy`, the unit tests, and
tarpaulin with `--fail-under 100`. Copy the workflow into your fork to keep
coverage locked at 100 %.

## License

MIT
