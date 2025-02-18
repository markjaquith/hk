---
outline: "deep"
---

# Configuration

## `hk.pkl`

hk is configured via `hk.pkl` which is written in [pkl-lang](https://pkl-lang.org/) from Apple.

Here's a basic `hk.pkl` file:

```pkl
amends "https://hk.jdx.dev/v0/hk.pkl"
import "https://hk.jdx.dev/v0/builtins/prettier.pkl"

`pre-commit` {
    // hooks can be manually defined
    ["eslint"] {
        // the files to run the hook on, if no files are matched, the hook will be skipped
        // this will filter the staged files and return the subset matching these globs
        glob = new { "*.js"; "*.ts" }
        // the command to run the hook on the files that makes no changes
        check = "eslint {{files}}"
        // the command to run the hook on the files that fixes them (used by default)
        fix = "eslint --fix {{files}}"
    }
    // hooks can also be specified with the builtins pkl library
    ["prettier"] = new prettier.Prettier {}
}
```

The first line (`amends`) is critical because that imports the base configuration pkl for extending.

### `min_hk_version: String`

The minimum version of hk that is required to run hk. hk will fail to start if its version is below the specified version.

```pkl
min_hk_version = "0.1.0"
```

### `<HOOK>`

Hooks are made up of steps. The hook themselves can be one of the following:

- `pre-commit`
- `pre-push`
- TODO: add more

### `<HOOK>.<STEP>`

Steps are the individual commands that make up a hook. They are executed in the order they are defined in parallel up to [`HK_JOBS`](/configuration#hk-jobs) at a time.

### `<HOOK>.<STEP>.profiles: Listing<String>`

Profiles are a way to enable/disable steps based on the current profile. The step will only be run the step's profile is in [`HK_PROFILE`](/configuration#hk-profile).

```pkl
`pre-commit` {
    ["prettier"] {
        profiles = new { "slow" }
    }
}
```

Profiles can be prefixed with `!` to disable them.

```pkl
`pre-commit` {
    ["prettier"] {
        profiles = new { "!slow" }
    }
}
```

### `<HOOK>.<STEP>.glob: Listing<String>`

Files the step should run on. By default this will only run this step if at least 1 staged file matches the glob patterns. If no patterns are provided, the step will always run.

### `<HOOK>.<STEP>.exclusive: bool`

Default: `false`

If true, this step will wait for any previous steps to finish before running. No other steps will start until this one finishes.

```pkl
`pre-commit` {
    ["prelint"] {
        exclusive = true // blocks other steps from starting until this one finishes
        check = "mise run prelint"
    }
    // ... other steps will run in parallel ...
    ["postlint"] {
        exclusive = true // wait for all previous steps to finish before starting
        check = "mise run postlint"
    }
}
```

### `<HOOK>.<STEP>.check: String`

A command to run for the hook that does not modify files. This typically is a "check" command like `eslint` or `prettier --check` that returns a non-zero exit code if there are errors.
Parallelization works better with run commands than fix commands as no files are being modified.

```pkl
`pre-commit` {
    ["prettier"] {
        check = "prettier --check {{files}}"
    }
}
```

Template variables:

- <code v-pre>{{files}}</code>: A list of files to run the hook on.

### `<HOOK>.<STEP>.fix: String`

A command to run for the hook that modifies files. This typically is a "fix" command like `eslint --fix` or `prettier --write`. Templates variables are the same as for `run`.

```pkl
`pre-commit` {
    ["prettier"] {
        fix = "prettier --write {{files}}"
    }
}
```

By default, hk will use `fix` commands but this can be overridden by setting [`HK_FIX=0`](/configuration#hk-fix) or running `hk run <HOOK> --run`.

### `<HOOK>.<STEP>.check_all: String`

A command to run for the hook that runs on all files. This is optional but if not specified hk will need to pass every file to the `check` command.

```pkl
`pre-commit` {
    ["prettier"] {
        check_all = "prettier --check ."
    }
}
```

### `<HOOK>.<STEP>.fix_all: String`

A command to run for the hook that runs on all files. This is optional but if not specified hk will need to pass every file to the `fix` command.

```pkl
`pre-commit` {
    ["prettier"] {
        fix_all = "prettier --write ."
    }
}
```

### Alternative Syntax

You can also write `hk.json|hk.yaml|hk.toml` as an alternative to pkl, however builtins will not be available.
This may go away in the future so let me know if you rely on it.

## Environment Variables

Environment variables can be used to configure hk.

### `HK_CACHE_DIR`

Type: `path`
Default: `~/.cache/hk`

The cache directory to use.

### `HK_CHECK_FIRST`

Type: `bool`
Default: `true`

If true, hk will run check on files first then run fix steps if check fails iff there are multiple fix steps with the same file in a matching glob pattern.

The reason for this is to make hk able to parallelize as much as possible. We can have as many check steps running in parallel against
the same file as we want without them interfering with each other—however we can't have 2 fix steps potentially writing to the same file. So we optimistically run the check steps in parallel, then if any fail we run their fix command potentially in series.

If this is disabled hk will have simpler logic that just uses fix steps in series in this situation.

### `HK_PROFILE`

Type: `string[]` (comma-separated list)

The profile(s) to use.

### `HK_FILE`

Type: `string`
Default: `hk.pkl` | `hk.toml` | `hk.yaml` | `hk.yml` | `hk.json`

The file to use for the configuration.

### `HK_FIX`

Type: `bool`
Default: `true`

If set to `false`, hk will not run fix steps.

### `HK_JOBS`

Type: `usize`
Default: `(number of cores)`

The number of jobs to run in parallel.

### `HK_LOG`

Type: `trace` | `debug` | `info` | `warn` | `error`
Default: `info`

The log level to use.

### `HK_LOG_FILE`

Type: `path`
Default: `~/.local/state/hk/hk.log`

The log file to use.

### `HK_LOG_FILE_LEVEL`

Type: `trace` | `debug` | `info` | `warn` | `error`
Default: `HK_LOG`

The log level to use for the log file.

### `HK_MISE`

Type: `bool`
Default: `false`

If set to `true`:

- When installing hooks with `hk install`, hk will use `mise x` to execute hooks which won't require activating mise to use mise tools
- When generating files with `hk generate`, hk will create a `mise.toml` file with hk configured

### `HK_SKIP_STEPS`

Type: `string[]` (comma-separated list)

A comma-separated list of step names to skip when running pre-commit and pre-push hooks.
For example: `HK_SKIP_STEPS=lint,test` would skip any steps named "lint" or "test".

### `HK_SKIP_HOOK`

Type: `string[]` (comma-separated list)
Default: `(empty)`

A comma-separated list of hook names to skip entirely. This allows you to disable specific git hooks from running.
For example: `HK_SKIP_HOOK=pre-commit,pre-push` would skip running those hooks completely.

This is useful when you want to temporarily disable certain hooks while still keeping them configured in your `hk.pkl` file.
Unlike `HK_SKIP_STEPS` which skips individual steps, this skips the entire hook and all its steps.

### `HK_STASH`

Type: `bool`
Default: `true`

If set to `false`, hk will not automatically stash unstaged changes before running hooks.

### `HK_STATE_DIR`

Type: `path`
Default: `~/.local/state/hk`

The state directory to use.
