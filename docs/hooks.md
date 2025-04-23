# Hooks

The following describes the behavior of git hooks that hk supports. Each linter provides a "check" and "fix" commands. "check" commands are read-only and can be run in parallel. "fix" commands can edit files and will block other "fix" or "check" commands from running at the same time. Note that hk does not enforce that "check" commands do not write to files for performance reasons however you still should try to follow this convention in order for hk to behave as expected.

It's the read/write locking behavior that hk makes use of in order to run hooks as fast as possible while still being safe.

A "group" is a collection of hook steps separated by steps with `exclusive = true`.

## `pre-commit`

Runs when `git commit` is run before the commit is created.

* Stashes any untracked/unstaged changes (disable with [`HK_STASH=0`](/configuration#hk-stash))
* Gathers list of files with staged changes (or all files if running `hk run pre-commit --all`)
* Runs linters and hook steps in parallel up to [`HK_JOBS`](/configuration#hk-jobs) at a time, with caveats:
  * `exclusive = true` hook steps will wait until all previous steps finished and block later steps from starting
  * if any hook step has any dependencies, hk will wait for them to complete before starting
  * hk will create read/write locks for each file to check/fix in the linters
  * if "fix" is set (default behavior) _and_ multiple linters in the same group are to edit the same file, hk will do one of the following:
    * if `stomp = true`, hk will grab read locks instead of write locks for the "fix" command. Use this if the tool itself has its own locking
      behavior or you simply don't care if the files may be written by multiple fix commands at the same time.
    * if `check_first = true` on the linter, hk will run the "check" command first with a read lock, if that fails, it will run the "fix" command with a write lock
      * if a `check_list_files` command is available on the linter, hk will use the output of that command to filter the list of files to get write locks for and call "fix" on.
    * if `check_first = false` on the linter, hk will run the "fix" command with write locks, blocking other linters from running
    * modified files are added to the git index
  * if "check" is set (because the linter does not have a "fix" command, [`HK_FIX=0`](/configuration#hk-fix) is set, or `hk check`), hk runs all linters in parallel. They should not be modifying files so this should be safe to do.
  * untracked/unstaged changes are unstashed
  * commit is allowed to run if no check/fix commands failed

## `pre-push`

Runs when `git push` is run before `git push` sends the changes to the remote repository.

```pkl
hooks = new {
    ["pre-push"] {
        ["check"] = new Check {}
    }
}
```

## `prepare-commit-msg`

Runs when `git commit` is run before the commit message is created. Useful for rendering a default commit message template.

```pkl
hooks = new {
    ["prepare-commit-msg"] {
        ["render-commit-msg"] {
            check = "echo 'default commit message' > {{commit_msg_file}}"
        }
    }
}

```

## `commit-msg`

Runs when `git commit` is run after the commit message is created. Useful for validating the commit message.

```pkl
hooks = new {
    ["commit-msg"] {
        ["validate-commit-msg"] {
            check = "grep -q '^(fix|feat|chore):' {{commit_msg_file}} || exit 1"
        }
    }
}
```
