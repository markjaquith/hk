# Configuration

hk is configured via `hk.pkl` which is written in [pkl-lang](https://pkl-lang.org/) from Apple.

## `hk.pkl`

Here's a basic `hk.pkl` file:

```pkl
amends "https://hk.jdx.dev/v0/hk.pkl"
import "https://hk.jdx.dev/v0/builtins.pkl" // optional

`pre-commit` {
    // hooks can be manually defined
    ["eslint"] {
        // the files to run the hook on, if no files are matched, the hook will be skipped
        // this will filter the staged files and return the subset matching these globs
        glob = new { "*.js"; "*.ts" }
        // the command to run the hook on the files that makes no changes
        run = "eslint {{files}}"
        // the command to run the hook on the files that fixes them (used by default)
        fix = "eslint --fix {{files}}"
    }
    // hooks can also be specified with the builtins pkl library
    ["prettier"] = new builtins.Prettier {}
}
```

The first line (`amends`) is critical because that imports the base configuration pkl for extending. The second line (`import`) imports
the builtins, so it's only necessary if actually using builtins. 

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

Steps are the individual commands that make up a hook. They are executed in the order they are defined in parallel up to [`HK_JOBS`](/environment_variables#hk-jobs) at a time.


### `<HOOK>.<STEP>.glob: Listing<String>`

Files the step should run on. By default this will only run this step if at least 1 staged file matches the glob patterns. If no patterns are provided, the step will always run.

### `<HOOK>.<STEP>.exclusive: bool`

Default: `false`

If true, this step will wait for any previous steps to finish before running. No other steps will start until this one finishes.

```pkl
`pre-commit` {
    ["prelint"] {
        exclusive = true // blocks other steps from starting until this one finishes
        run = "mise run prelint"
    }
    // ... other steps will run in parallel ...
    ["postlint"] {
        exclusive = true // wait for all previous steps to finish before starting
        run = "mise run postlint"
    }
}
```

### `<HOOK>.<STEP>.run: String`

A command to run for the hook that does not modify files. This typically is a "check" command like `eslint` or `prettier --check` that returns a non-zero exit code if there are errors.
Parallelization works better with run commands than fix commands as no files are being modified.

```pkl
`pre-commit` {
    ["prettier"] {
        run = "prettier --check {{files}}"
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

By default, hk will use `fix` commands but this can be overridden by setting [`HK_FIX=0`](/environment_variables#hk-fix) or running `hk run <HOOK> --run`.

### `<HOOK>.<STEP>.run_all: String`

A command to run for the hook that runs on all files. This is optional but if not specified hk will need to pass every file to the `run` command.

```pkl
`pre-commit` {
    ["prettier"] {
        run_all = "prettier --check ."
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

## Alternative Syntax

You can also write `hk.json|hk.yaml|hk.toml` as an alternative to pkl, however builtins will not be available.
This may go away in the future so let me know if you rely on it.
