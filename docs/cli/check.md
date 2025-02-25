# `hk check`

- **Usage**: `hk check [FLAGS]`
- **Aliases**: `c`

Fixes code

## Flags

### `-a --all`

Run on all files instead of just staged files

### `--linter... <LINTER>`

Run on specific linter(s)

### `--stash`

Force stashing even if it's disabled via HK_STASH

### `--from-ref <FROM_REF>`

Start reference for checking files (requires --to-ref)

### `--to-ref <TO_REF>`

End reference for checking files (requires --from-ref)
