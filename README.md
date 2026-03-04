# winget-bundle

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**winget-bundle** is a package manager wrapper for Windows, inspired by [Homebrew's `bundle` subcommand](https://docs.brew.sh/Manpage#bundle-subcommand).
Declare all your packages in a single `Bundlefile`, then run one command to install or upgrade everything — across both [winget](https://github.com/microsoft/winget-cli) and [Scoop](https://scoop.sh/).

---

## Features

- **Declarative** — list all packages in one `Bundlefile`
- **Multi-source** — supports `winget`, `msstore`, and `scoop` sources in a single file
- **Lock file** — a `Bundlefile.lock` records the installed state of `winget` packages
- **Smart upgrade** — upgrades outdated packages automatically; opt out per-package or globally
- **Cleanup** — uninstalls packages that have been removed from the `Bundlefile` (dry-run by default)

---

## Requirements

- Windows 10 / 11
- [winget](https://aka.ms/winget) (for `winget` and `msstore` sources)
- [Scoop](https://scoop.sh/) (for `scoop` source, optional)
- [Rust toolchain](https://rustup.rs/) (to build from source)

---

## Installation

### From source

```sh
cargo install --path .
```

---

## Bundlefile

`winget-bundle` looks for your `Bundlefile` in the following locations, in order:

| Condition | Path |
|---|---|
| `$env:XDG_CONFIG_HOME` is set | `$env:XDG_CONFIG_HOME\winget-bundle\Bundlefile` |
| *(default)* | `$env:USERPROFILE\.Bundlefile` |

### Syntax

```
# This is a comment
<source> "<id>" [, <key>: <value> ...]
```

| Field | Description |
|---|---|
| `source` | One of `winget`, `msstore`, or `scoop` |
| `id` | The package identifier (or name for `msstore`) |
| `name` | *(optional)* Human-readable label shown in output |
| `no_upgrade` | *(optional, bool)* Skip upgrade for this package if `true` |

### Example

```
# Editors
winget "Microsoft.VisualStudioCode", name: "VS Code"
winget "Neovim.Neovim"

# Terminal
winget "Microsoft.WindowsTerminal", no_upgrade: true

# Microsoft Store
msstore "9N0DX20HK701", name: "Windows Terminal (Store)"

# Scoop
scoop "git"
scoop "ripgrep"
scoop "fd"
```

---

## Usage

### Install (default command)

Install all packages declared in the `Bundlefile`. Outdated packages are upgraded unless opted out.

```powershell
winget-bundle
# or explicitly:
winget-bundle install
```

#### Upgrade behavior

By default, outdated packages **are upgraded** during `install`.  
Set the environment variable `$env:WINGET_BUNDLE_NO_UPGRADE` to disable upgrades globally:

```powershell
$env:WINGET_BUNDLE_NO_UPGRADE = "1"
winget-bundle
```

Use the flags below to override this behavior on a per-run basis:

| Flag | Description |
|---|---|
| `--upgrade` | Always upgrade, even if `WINGET_BUNDLE_NO_UPGRADE` is set |
| `--no-upgrade` | Never upgrade, regardless of env var |

```powershell
winget-bundle install --upgrade
winget-bundle install --no-upgrade
```

Per-package opt-out is also available via the `no_upgrade` option in the `Bundlefile`.

---

### Cleanup

Uninstall packages that are **no longer present** in the `Bundlefile`.

```powershell
# Dry run — shows what would be removed
winget-bundle cleanup

# Actually uninstall
winget-bundle cleanup --force
```

---

### Help

```powershell
winget-bundle --help
winget-bundle install --help
winget-bundle cleanup --help
```

---

## Lock File

After a successful install, `winget-bundle` writes a `Bundlefile.lock` file next to your `Bundlefile`.  
This TOML file records the installed winget packages, and is used by `cleanup` to track what was installed by `winget-bundle`.

> **Note:** `msstore` and `scoop` packages are excluded from the lock file. `msstore` packages are managed directly through winget without version tracking, and `scoop` packages are excluded because `winget-bundle` queries Scoop's own state directly to determine what is installed.

---

## Environment Variables

| Variable | Description |
|---|---|
| `USERPROFILE` | Used to locate `%USERPROFILE%\.Bundlefile` when `XDG_CONFIG_HOME` is not set |
| `WINGET_BUNDLE_NO_UPGRADE` | When set, disables automatic upgrades during `install` (same as `--no-upgrade`) |
| `XDG_CONFIG_HOME` | If set, the `Bundlefile` is read from `$XDG_CONFIG_HOME\winget-bundle\Bundlefile` |

---

## Contributing

If you find a bug or have a question, feel free to [open an issue](https://github.com/progre/winget-bundle/issues).

---

## License

This project is licensed under the [MIT License](LICENSE).
