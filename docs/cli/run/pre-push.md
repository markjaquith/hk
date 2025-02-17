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

### `-r --run`

Run run command instead of fix command
