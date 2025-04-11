# `hk run pre-commit`

- **Usage**: `hk run pre-commit [FLAGS]`
- **Aliases**: `pc`

Sets up git hooks to run hk

## Flags

### `-a --all`

Run on all files instead of just staged files

### `-f --fix`

Run fix command instead of run command This is the default behavior unless HK_FIX=0

### `-c --check`

Run run command instead of fix command

### `--linter... <LINTER>`

Run on specific linter(s)

### `--from-ref <FROM_REF>`

Start reference for checking files (requires --to-ref)

### `--to-ref <TO_REF>`

End reference for checking files (requires --from-ref)
