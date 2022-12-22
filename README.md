# SNAFU Lints

This uses [dylint][] to add SNAFU-specific lints.

[dylint]: https://github.com/trailofbits/dylint

## Installation

Install [dylint][] per their instructions, then add the SNAFU lints to
your dylint configuration:

```toml
[workspace.metadata.dylint]
libraries = [
    { git = "https://github.com/shepmaster/snafu-lints", pattern = "perf/*" },
]
```

## Lints

- [`snafu_context_fun_call`](./perf/snafu_context_fun_call)
