# `hk run pre-push`

- **Usage**: `hk run pre-push [FLAGS] <REMOTE> <URL>`
- **Aliases**: `ph`

Sets up git hooks to run hk

## Arguments

### `<REMOTE>`

Remote name

### `<URL>`

Remote URL

## Flags

### `-a --all`

Run on all files instead of just staged files

### `-f --fix`

Run fix command instead of run command This is the default behavior unless HK_FIX=0

### `-c --check`

Run check command instead of fix command

### `--stash`

Force stashing even if it's disabled via HK_STASH
