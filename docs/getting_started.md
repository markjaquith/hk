# Getting Started

A tool for running hooks on files in a git repository.

> [!CAUTION]
> This is a work in progress.

## Installation

Use [mise-en-place](https://github.com/jdx/mise) to install hk:

```
mise use hk
hk --version
```

:::tip
mise-en-place integrates well with hk. Features common in similar git-hook managers like dependency management, task dependencies, and env vars can be provided by mise.

See [mise integration](/mise_integration) for more information.
:::

Or install from source with `cargo`:

```
cargo install hk
```

## Project Setup

Use `hk generate` to generate a `hk.pkl` file:

```
hk generate
```

## `hk.pkl`

This will generate a `hk.pkl` file in the root of the repository, here's an example `hk.pkl` with eslint and prettier hooks:

```pkl
amends "https://hk.jdx.dev/v0/hk.pkl"
import "https://hk.jdx.dev/v0/builtins.pkl"

`pre-commit` {
    // hooks can be manually defined
    ["eslint"] {
        // the files to run the hook on, if no files are matched, the hook will be skipped
        // this will filter the staged files and return the subset matching these globs
        glob = new { "*.js"; "*.ts" }
        // a command that returns non-zero to fail the step
        run = "eslint {{files}}"
    }
    // hooks can also be specified with the builtins pkl library
    ["prettier"] = new builtins.Prettier {}
}
```

See [configuration](/configuration) for more information on the `hk.pkl` file.

## Usage

Inside a git repository with a `hk.pkl` file, run:

```
hk install
```

This will install the hooks for the repository like `pre-commit` and `pre-push` if they are defined in `hk.pkl`. Running `git commit` would now run the `pre-commit` steps defined above in our example.

## Running Hooks

To explicitly run the hooks without going through git, use the `hk run` command.

```
hk run pre-commit
```

This will run the `pre-commit` hooks for the repository. This will run against all files that are staged for commit. To run against all files in the repository, use the `--all` flag.

```
hk run pre-commit --all
```

To run a specific step, use the `--step` flag.

```
hk run pre-commit --step eslint
```
