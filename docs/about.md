# About

hk is built by [@jdx](https://github.com/jdx).

## Why does this exist?

I wanted a git hook manager that was fast and I didn't see any
existing tools that integrated well enough with linters to actually perform as
fast as they could.

I recognized that in order for git hooks to perform fast, they need to run in parallel. That said, since linters also often (but not always) modify files, it's
not possible to naively run them in parallel or they will stomp on each other.

That doesn't mean we have to run everything in series though. A lint manager which
was aware of how linters will behave could grab read/write locks and only grab
write locks for a short period of time would ultimately enable a lint manager to
lint a codebase or set of changes very quickly.

It takes a bit more time to implement linters in hk than it does with similar tools
in order to take advantage of this benefit which is one reason why it comes with
many [builtins](https://github.com/jdx/hk/tree/main/pkl/builtins) to get you started with common tools.

## Benchmarks

These are basic benchmarks on the hk codebase which is admittedly a simple codebase. Real-world benchmarks with large codebases will likely show hk to be
much faster when it is able to take advantage of its deep parallelism capabilities.

![benchmarks](./public/benchmark.png)

## Roadmap

There isn't one yet. This project will hit 1.0 when I feel it has good enough parity that almost anyone would be able to switch from lefthook or pre-commit to hk.

Until then, expect frequent breaking changes and experimentation as I try to identify the proper behavior of hk.

## Contributing

Contributions are welcome! Please open an issue or submit a PR.
