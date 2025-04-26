---
outline: "deep"
---

# Configuration

## `hk.pkl`

hk is configured via `hk.pkl` which is written in [pkl-lang](https://pkl-lang.org/) from Apple.

Here's a basic `hk.pkl` file:

```pkl
amends "package://github.com/jdx/hk/releases/download/v0.8.5/hk@0.8.5#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v0.8.5/hk@0.8.5#/Builtins.pkl"

local linters {
    // linters can be manually defined
    ["eslint"] {
        // the files to run the linter on, if no files are matched, the linter will be skipped
        // this will filter the staged files and return the subset matching these globs
        glob = List("*.js", "*.ts")
        // these files will be staged after the fix step modifies them
        stage = List("*.js", "*.ts")
        // the command to run that makes no changes
        check = "eslint {{files}}"
        // the command to run that fixes the files (used by default)
        fix = "eslint --fix {{files}}"
    }
    // linters can also be specified with the Builtins pkl library
    ["prettier"] = Builtins.prettier
}

hooks {
    ["pre-commit"] {
        fix = true           // runs the fix step to make modifications
        stash = "patch-file" // stashes unstaged changes before running fix steps
        steps = linters
    }
    ["pre-push"] {
        steps = linters
    }
    // "fix" and "check" are special steps for `hk fix` and `hk check` commands
    ["fix"] {
        fix = true
        steps = linters
    }
    ["check"] {
        steps = linters
    }
}
```

The first line (`amends`) is critical because that imports the base configuration pkl for extending.

### `hooks.<HOOK>.steps`

Steps are the individual commands that make up a hook. They are executed in the order they are defined in parallel up to [`HK_JOBS`](/configuration#hk-jobs) at a time.

### `hooks.<HOOK>.steps.<STEP>.glob: List<String>`

Files the step should run on. By default this will only run this step if at least 1 staged file matches the glob patterns. If no patterns are provided, the step will always run.

### `hooks.<HOOK>.steps.<STEP>.check: String`

A command to run that does not modify files. This typically is a "check" command like `eslint` or `prettier --check` that returns a non-zero exit code if there are errors.
Parallelization works better with check commands than fix commands as no files are being modified.

```pkl
hooks {
    ["pre-commit"] {
        ["prettier"] {
            check = "prettier --check {{files}}"
        }
    }
}
```

Template variables:

- <code v-pre>{{files}}</code>: A list of files to run the linter on.

### `hooks.<HOOK>.steps.<STEP>.check_list_files: String`

A command that returns a list of files that need fixing. This is used to optimize the fix step when `check_first` is enabled. Instead of running the fix command on all files, it will only run on files that need fixing.

```pkl
hooks {
    ["pre-commit"] {
        ["prettier"] {
            check_list_files = "prettier --list-different {{files}}"
        }
    }
}
```

### `hooks.<HOOK>.steps.<STEP>.check_diff: String`

A command that shows the diff of what would be changed. This is an alternative to `check` that can provide more detailed information about what would be changed.

### `hooks.<HOOK>.steps.<STEP>.fix: String`

A command to run that modifies files. This typically is a "fix" command like `eslint --fix` or `prettier --write`. Templates variables are the same as for `check`.

```pkl
local linters {
    ["prettier"] {
        fix = "prettier --write {{files}}"
    }
}
```

By default, hk will use `fix` commands but this can be overridden by setting [`HK_FIX=0`](/configuration#hk-fix) or running `hk run <HOOK> --run`.

### `hooks.<HOOK>.steps.<STEP>.check_first: bool`

Default: `true`

If true, hk will run the check step first and only run the fix step if the check step fails.

### `hooks.<HOOK>.steps.<STEP>.batch: bool`

Default: `false`

If true, hk will run the linter on batches of files instead of all files at once. This takes advantage of parallel processing for otherwise single-threaded linters like eslint and prettier.

```pkl
local linters {
    ["eslint"] {
        batch = true
    }
}
```

### `hooks.<HOOK>.steps.<STEP>.stomp: bool`

Default: `false`

If true, hk will get a write lock instead of a read lock when running fix/fix_all. Use this if the tool has its own locking mechanism or you simply don't care if files may be written to by multiple linters simultaneously.

### `hooks.<HOOK>.steps.<STEP>.workspace_indicator: String`

If set, run the linter on workspaces only which are parent directories containing this filename. This is useful for tools that need to be run from a specific directory, like a project root.

```pkl
local linters {
    ["cargo-clippy"] {
        workspace_indicator = "Cargo.toml"
    }
}
```

### `hooks.<HOOK>.steps.<STEP>.prefix: String`

If set, run the linter scripts with this prefix, e.g.: "mise exec --" or "npm run".

```pkl
local linters {
    ["eslint"] {
        prefix = "npm run"
    }
}
```

### `hooks.<HOOK>.steps.<STEP>.dir: String`

If set, run the linter scripts in this directory.

```pkl
local linters {
    ["eslint"] (Builtins.eslint) {
        dir = "frontend"
    }
}
```

### `hooks.<HOOK>.steps.<STEP>.profiles: List<String>`

Profiles are a way to enable/disable linters based on the current profile. The linter will only run if its profile is in [`HK_PROFILE`](/configuration#hk-profile).

```pkl
local linters {
    ["prettier"] (Builtins.prettier) {
        profiles = List("slow")
    }
}
```

Profiles can be prefixed with `!` to disable them.

```pkl
local linters {
    ["prettier"] (Builtins.prettier) {
        profiles = List("!slow")
    }
}
```

### `hooks.<HOOK>.steps.<STEP>.depends: List<String>`

A list of steps that must finish before this step can run.

```pkl
hooks {
    ["pre-commit"] {
        steps {
            ["prettier"] {
                depends = List("eslint")
            }
        }
    }
}
```

### `hooks.<HOOK>`

Hooks define when and how linters are run. See [hooks](/hooks) for more information.

### `hooks.<HOOK>.<STEP>.stage: List<String>`

A list of globs of files to add to the git index after running a fix step.

```pkl
hooks {
    ["pre-commit"] {
        steps {
            ["prettier"] {
                stage = List("*.js", "*.ts")
            }
        }
    }
}
```

### `hooks.<HOOK>.steps.<STEP>.exclusive: bool`

Default: `false`

If true, this step will wait for any previous steps to finish before running. No other steps will start until this one finishes. Under
the hood this groups the previous steps into a group.

```pkl
hooks {
    ["pre-commit"] {
        steps {
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
    }
}

### `hooks.<HOOK>.steps.<GROUP>`

```pkl
hooks {
    ["pre-commit"] {
        steps {
            ["build"] = new Group {
                steps = new Mapping<String, Step> {
                    ["ts"] = new Step {
                        fix = "tsc -b"
                    }
                    ["rs"] = new Step {
                        fix = "cargo build"
                    }
                }
            }
            // these steps will run in parallel after the build group finishes
            ["lint"] = new Group {
                steps = new Mapping<String, Step> {
                    ["prettier"] = new Step {
                        check = "prettier --check {{files}}"
                    }
                    ["eslint"] = new Step {
                        check = "eslint {{files}}"
                    }
                }
            }
        }
    }
}
```

### `env: Mapping<String, String>`

Environment variables can be set in hk.pkl for configuring hk or the linters.

```pkl
env {
    ["HK_FAIL_FAST"] = "0"
    ["NODE_ENV"] = "production"
}
```

### Alternative Syntax

You can also write `hk.json|hk.yaml|hk.toml` as an alternative to pkl, however builtins will not be available.
This may go away in the future so let me know if you rely on it.
