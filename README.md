rust-cgi
========
[![Crate](https://img.shields.io/crates/v/rust-cgi.svg)](https://crates.io/crates/rust-cgi)
[![License](https://img.shields.io/crates/l/rust-cgi.svg)](LICENSE)

Easily create CGI (Common Gateway Interface) programs in Rust, based on
[`http`](https://crates.io/crates/http) types.

This repository is a fork of the unmaintained https://github.com/amandasaurus/rust-cgi,
which was published to crates.io as the `cgi` crate.

Installation & Usage
--------------------

`Cargo.toml`:

```toml
[dependencies]
rust-cgi = "0.6"
```

Use the `cgi_main!` macro, with a function that takes a `rust_cgi::Request` and returns a
`rust_cgi::Response`.

```rust
extern crate rust_cgi as cgi;

cgi::cgi_main! { |request: cgi::Request| -> cgi::Response {
     cgi::text_response(200, "Hello World")
} }
```

If your function returns a `Result`, you can use `cgi_try_main!`:

```rust
extern crate rust_cgi as cgi;

cgi::cgi_try_main! { |request: cgi::Request| -> Result<cgi::Response, String> {
    let greeting = std::fs::read_to_string("greeting.txt").map_err(|_| "Couldn't open file")?;

    Ok(cgi::text_response(200, greeting))
} }
```

It will parse and extract the CGI environmental variables, and the HTTP request body to create
`Request<u8>`, call your function to create a response, and convert your `Response` into the
correct format and print to stdout. If this program is not called as CGI (e.g. missing
required environmental variables), it will gracefully fall back to using reasonable values
(although the values themselves may be subject to change).

It is also possible to call the `rust_cgi::handle` function directly inside your `main` function:

```rust
extern crate rust_cgi as cgi;

fn main() { cgi::handle(|request: cgi::Request| -> cgi::Response {
    cgi::html_response(200, "<html><body><h1>Hello World!</h1></body></html>")
})}
```

Response Shortcuts
------------------

Several shortcuts create shortcuts easily:

- `rust_cgi:empty_response(status_code)` - A HTTP Reponse with no body and that HTTP
status code, e.g. `return rust_igi::empty_response(404);` to return a
[HTTP 404 Not Found](https://en.wikipedia.org/wiki/HTTP_404).

- `rust_cgi::html_response(status_code, text)` - Converts `text` to bytes (UTF8) and
sends that as the body with that `status_code` and HTML `Content-Type` header.

- `rust_cgi::string_response(status_code, text)` - Converts `text` to bytes (UTF8),
and sends that as the body with that `status_code` but no `Content-Type` header.

- `rust_cgi::binary_response(status_code, content_type, blob)` - Sends `blob` with
that status code and the provided content type header.

Re-exports
----------

`http` is re-exported, (as `rust_cgi::http`).

`rust_cgi::Response`/`Request` are `http::Response<Vec<u8>>`/`Request<Vec<u8>>`.

Running locally
---------------

Python provides a simple CGI webserver you can use to run your scripts. The
binaries must be in a `cgi-bin` directory, so you'll need to create that
directory and copy your binary into it. Given a project named `example`, run
this in your project root directory (i.e. where `Cargo.toml` is):

```shell
mkdir cgi-bin
cargo build
cp target/debug/example cgi-bin/example
python3 -m http.server --cgi
```

and then open http://localhost:8000/cgi-bin/example.

MSRV policy
-----------

Currently the minimum supported Rust version (MSRV) is 1.51.0.
MSRV increases will be kept to a minimum, and will always be accompanied with a minor version bump.

See also
--------

- [Rustdoc for this crate](https://docs.rs/rust-cgi/latest/rust_cgi/)
- [http crate](https://github.com/hyperium/http)
- [RFC 3875 - The Common Gateway Interface (CGI) v1.1](https://tools.ietf.org/html/rfc3875)

Why?
----

CGI is old, and easy to deploy. Just drop a binary in the right place, and
Apache (or whatever) will serve it up. Rust is fast, so for simple things,
there should be less downsides to spinning up a custom HTTP server.
