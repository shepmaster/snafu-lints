# `snafu_context_fun_call`

## What it does

Warns about function and method calls inside of context selector fields used
with the `context` method.

## Why is this bad?

Context selectors passed to `context` are eagerly evaluated even when
no error has occurred. If the function call is expensive, the work
performed may be wasted.

## Known problems

None.

## Example

```rust
fn example(r: Result<i32, std::io::Error>) -> Result<i32, MyError> {
    let useful_data = "useful";

    r.context(MySnafu { useful_data: useful_data.to_string() });
    // `useful_data.to_string()` will always allocate a `String`, even
    // when no error has occurred
}

#[derive(Debug, Snafu)]
struct MyError {
    source: std::io::Error,
    useful_data: String,
}
```

Use instead:

```rust
fn example(r: Result<i32, std::io::Error>) -> Result<i32, MyError> {
    let useful_data = "useful";

    r.context(MySnafu { useful_data });
    // The context selector will call `Into::into` on each field when
    // an error is present, converting the `&str` into a `String` only
    // when needed.
}

#[derive(Debug, Snafu)]
struct MyError {
    source: std::io::Error,
    useful_data: String,
}
```

## Configuration

- `allow`: An array of strings, each the fully-qualified path to a
  function or method. These functions and methods should be cheap
  enough that unconditionally calling them is not a performance
  concern.

```toml
[snafu_context_fun_call]
allow = [
    "http::header::name::HeaderName::as_str",
]
```
