# Environment Variables

Environment variables can be used to configure hk.

## `HK_CACHE_DIR`

Default: `~/.cache/hk`

The cache directory to use.

## `HK_FIX`

Default: `true`

If set to `false`, hk will not run fix steps.

## `HK_JOBS`

Default: `(number of cores)`

The number of jobs to run in parallel.

## `HK_LOG`

Default: `info`

The log level to use.

## `HK_LOG_FILE`

Default: `~/.local/state/hk/hk.log`

The log file to use.

## `HK_LOG_FILE_LEVEL`

Default: `HK_LOG`

The log level to use for the log file.

## `HK_MISE`

Default: `false`

If set to `true`:
- When installing hooks with `hk install`, hk will use `mise x` to execute hooks which won't require activating mise to use mise tools
- When generating files with `hk generate`, hk will create a `mise.toml` file with hk configured

## `HK_SKIP_STEPS`

Default: (empty)

A comma-separated list of step names to skip when running pre-commit and pre-push hooks.
For example: `HK_SKIP_STEPS=lint,test` would skip any steps named "lint" or "test".

## `HK_STASH`

Default: `true`

If set to `false`, hk will not automatically stash unstaged changes before running hooks.

## `HK_STATE_DIR`

Default: `~/.local/state/hk`

The state directory to use.
