//! Easily create CGI (RFC 3875) programmes in Rust based on hyper's [`http`](https://github.com/hyperium/http) types.
//!
//! # Installation & Usage
//!
//! `Cargo.toml`:
//!
//! ```cargo,ignore
//! [dependencies]
//! rust_cgi = "0.3"
//! ```
//!
//!
//! Use the [`cgi_main!`](macro.cgi_main.html) macro, with a function that takes a `rust_cgi::Request` and returns a
//! `rust_cgi::Response`.
//!
//! ```rust
//! extern crate rust_cgi as cgi;
//!
//! cgi::cgi_main! { |request: cgi::Request| -> cgi::Response {
//!      cgi::text_response(200, "Hello World")
//! } }
//! ```
//!
//! If your function returns a `Result`, you can use [`cgi_try_main!`](macro.cgi_try_main.html):
//!
//! ```rust
//! extern crate rust_cgi as cgi;
//!
//! cgi::cgi_try_main! { |request: cgi::Request| -> Result<cgi::Response, String> {
//!     let greeting = std::fs::read_to_string("greeting.txt").map_err(|_| "Couldn't open file")?;
//!
//!     Ok(cgi::text_response(200, greeting))
//! } }
//! ```
//!
//! It will parse & extract the CGI environmental variables, and the HTTP request body to create
//! `Request<u8>`, call your function to create a response, and convert your `Response` into the
//! correct format and print to stdout.
//!
//! It is also possible to call the `rust_cgi::handle` ro `rust_cgi::try_handle` function directly
//! inside your `main` function:
//!
//! ```rust,ignore
//! extern crate rust_cgi as cgi;
//!
//! fn main() { cgi::handle(|request: cgi::Request| -> cgi::Response {
//!      cgi::empty_response(404)
//! })}
//! ```
//!
//! Several shortcut functions are provided (such as [`html_response`](fn.html_response.html)/[`binary_response`](fn.binary_response.html))

use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt::Debug;
use std::io::{Read, Write};

pub extern crate http;

/// A `Vec<u8>` Request from http
pub type Request = http::Request<Vec<u8>>;

/// A `Vec<u8>` Response from http
pub type Response = http::Response<Vec<u8>>;

fn handle_with_io<F, R, W>(func: F, mut stdin: R, mut stdout: W)
where
    F: FnOnce(Request) -> Response,
    R: Read,
    W: Write,
{
    let env_vars: HashMap<String, String> = std::env::vars().collect();

    // How many bytes do we have to read for request body
    // A general stdin().read_to_end() can block if the webserver doesn't close things
    let content_length: usize = env_vars
        .get("CONTENT_LENGTH")
        .and_then(|cl| cl.parse::<usize>().ok())
        .unwrap_or(0);

    let mut stdin_contents = vec![0; content_length];
    stdin.read_exact(&mut stdin_contents).unwrap();

    let request = parse_request(env_vars, stdin_contents);

    let response = func(request);

    let output = serialize_response(response);

    stdout.write_all(&output).unwrap();
}

fn try_handle_with_io<E, F, R, W, X>(func: F, stdin: R, stdout: W, mut stderr: X)
where
    E: Debug,
    F: FnOnce(Request) -> Result<Response, E>,
    R: Read,
    W: Write,
    X: Write,
{
    handle_with_io(
        |request: Request| match func(request) {
            Ok(resp) => resp,
            Err(err) => {
                writeln!(stderr, "{:?}", err).unwrap_or_else(|_| eprintln!("{:?}", err));
                empty_response(500)
            }
        },
        stdin,
        stdout,
    )
}

/// Call a function as a CGI programme.
///
/// This should be called from a `main` function.
/// Parse & extract the CGI environmental variables, and HTTP request body,
/// to create `Request`, and convert your `Response` into the correct format and
/// print to stdout.
pub fn handle<F>(func: F)
where
    F: FnOnce(Request) -> Response,
{
    handle_with_io(func, std::io::stdin(), std::io::stdout())
}

/// Call a function as a CGI programme.
///
/// Same as `handle`, but takes a function that returns a `Result`, and
/// in case of error, it writes the error to stderr, and returns an empty
/// 500 response.
pub fn try_handle<E, F>(func: F)
where
    E: Debug,
    F: FnOnce(Request) -> Result<Response, E>,
{
    try_handle_with_io(func, std::io::stdin(), std::io::stdout(), std::io::stderr())
}

#[macro_export]
/// Create a `main` function for a CGI script
///
/// Use the `cgi_main` macro, with a function that takes a `rust_cgi::Request` and returns a
/// `rust_cgi::Response`.
///
/// ```rust
/// extern crate rust_cgi as cgi;
///
/// cgi::cgi_main! { |request: cgi::Request| -> cgi::Response {
///     cgi::empty_response(200)
/// } }
/// ```
macro_rules! cgi_main {
    ( $func:expr ) => {
        fn main() {
            rust_cgi::handle($func);
        }
    };
}

#[macro_export]
/// Create a CGI main function based on a function which returns a `Result<rust_cgi::Response, _>`
///
/// If the inner function returns an `Ok(...)`, that will be unwrapped & returned. If there's an
/// error, it will be printed (`{:?}`) to stderr (which apache doesn't sent to the client, but
/// saves to a log file), and an empty `HTTP 500 Server Error` response is sent instead.
///
/// # Example
///
/// ```rust
/// extern crate rust_cgi as cgi;
///
/// cgi::cgi_try_main! { |request: cgi::Request| -> Result<cgi::Response, String> {
///     let f = std::fs::read_to_string("greeting.txt").map_err(|_| "Couldn't open file")?;
///
///     Ok(cgi::text_response(200, f))
/// } }
/// ```
macro_rules! cgi_try_main {
    ( $func:expr ) => {
        fn main() {
            rust_cgi::try_handle($func);
        }
    };
}

pub fn err_to_500<E>(res: Result<Response, E>) -> Response {
    res.unwrap_or(empty_response(500))
}

/// A HTTP Reponse with no body and that HTTP status code, e.g. `return rust_cgi::empty_response(404);`
/// to return a [HTTP 404 Not Found](https://en.wikipedia.org/wiki/HTTP_404).
pub fn empty_response<T>(status_code: T) -> Response
where
    http::StatusCode: TryFrom<T>,
    <http::StatusCode as TryFrom<T>>::Error: Into<http::Error>,
{
    http::response::Builder::new()
        .status(status_code)
        .body(vec![])
        .unwrap()
}

/// Converts `text` to bytes (UTF8) and sends that as the body with that `status_code` and HTML
/// `Content-Type` header (`text/html`)
pub fn html_response<T, S>(status_code: T, body: S) -> Response
where
    http::StatusCode: TryFrom<T>,
    <http::StatusCode as TryFrom<T>>::Error: Into<http::Error>,
    S: Into<String>,
{
    let body: Vec<u8> = body.into().into_bytes();
    http::response::Builder::new()
        .status(status_code)
        .header(http::header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header(
            http::header::CONTENT_LENGTH,
            format!("{}", body.len()).as_str(),
        )
        .body(body)
        .unwrap()
}

/// Convert to a string and return that with the status code
pub fn string_response<T, S>(status_code: T, body: S) -> Response
where
    http::StatusCode: TryFrom<T>,
    <http::StatusCode as TryFrom<T>>::Error: Into<http::Error>,
    S: Into<String>,
{
    let body: Vec<u8> = body.into().into_bytes();
    http::response::Builder::new()
        .status(status_code)
        .header(
            http::header::CONTENT_LENGTH,
            format!("{}", body.len()).as_str(),
        )
        .body(body)
        .unwrap()
}

/// Serves this content as `text/plain` text response, with that status code
///
/// ```rust,ignore
/// extern crate rust_cgi as cgi;
///
/// cgi::cgi_main! { |request: cgi::Request| -> cgi::Response {
///   cgi::text_response(200, "Hello world");
/// } }
/// ```
pub fn text_response<T, S>(status_code: T, body: S) -> Response
where
    http::StatusCode: TryFrom<T>,
    <http::StatusCode as TryFrom<T>>::Error: Into<http::Error>,
    S: Into<String>,
{
    let body: Vec<u8> = body.into().into_bytes();
    http::response::Builder::new()
        .status(status_code)
        .header(
            http::header::CONTENT_LENGTH,
            format!("{}", body.len()).as_str(),
        )
        .header(http::header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(body)
        .unwrap()
}

/// Sends  `blob` with that status code, and optional content type, `None` for no `Content-Type`
/// header to be set.
///
/// No `Content-Type` header:
///
/// ```rust,ignore
/// rust_cgi::binary_response(200, None, vec![1, 2]);
/// ```
///
/// Send an image:
///
/// ```rust,ignore
/// rust_cgi::binary_response(200, "image/png", vec![1, 2]);
/// ```
///
/// Send a generic binary blob:
///
/// ```rust,ignore
/// rust_cgi::binary_response(200, "application/octet-stream", vec![1, 2]);
/// ```
pub fn binary_response<'a, T>(
    status_code: T,
    content_type: impl Into<Option<&'a str>>,
    body: Vec<u8>,
) -> Response
where
    http::StatusCode: TryFrom<T>,
    <http::StatusCode as TryFrom<T>>::Error: Into<http::Error>,
{
    let content_type: Option<&str> = content_type.into();

    let mut response = http::response::Builder::new().status(status_code).header(
        http::header::CONTENT_LENGTH,
        format!("{}", body.len()).as_str(),
    );

    if let Some(ct) = content_type {
        response = response.header(http::header::CONTENT_TYPE, ct);
    }

    response.body(body).unwrap()
}

fn exe_url() -> String {
    // maybe use http::uri::Uri instead.
    // note: the http crate that this outputs to only accepts http/https URLs,
    // so we're going to leave off the scheme entirely
    match std::env::current_exe() {
        Ok(p) => p.to_string_lossy().into_owned(),
        Err(_) => String::new(),
    }
}

fn parse_request(env_vars: HashMap<String, String>, stdin: Vec<u8>) -> Request {
    let mut req = http::Request::builder();

    req = req.method(env_vars.get("REQUEST_METHOD").map_or("GET", String::as_str));
    let mut uri = env_vars
        .get("SCRIPT_NAME")
        .map_or_else(exe_url, String::clone);

    if env_vars.contains_key("QUERY_STRING") {
        uri.push_str("?");
        uri.push_str(&env_vars["QUERY_STRING"]);
    }
    req = req.uri(uri.as_str());

    if let Some(v) = env_vars.get("SERVER_PROTOCOL") {
        if v == "HTTP/0.9" {
            req = req.version(http::version::Version::HTTP_09);
        } else if v == "HTTP/1.0" {
            req = req.version(http::version::Version::HTTP_10);
        } else if v == "HTTP/1.1" {
            req = req.version(http::version::Version::HTTP_11);
        } else if v == "HTTP/2.0" {
            req = req.version(http::version::Version::HTTP_2);
        } else {
            unimplemented!("Unsupport HTTP SERVER_PROTOCOL {:?}", v);
        }
    }

    for key in env_vars.keys().filter(|k| k.starts_with("HTTP_")) {
        let header: String = key
            .chars()
            .skip(5)
            .map(|c| if c == '_' { '-' } else { c })
            .collect();
        req = req.header(header.as_str(), env_vars[key].as_str().trim());
    }

    req = add_header(req, &env_vars, "AUTH_TYPE", "X-CGI-Auth-Type");
    req = add_header(req, &env_vars, "CONTENT_LENGTH", "X-CGI-Content-Length");
    req = add_header(req, &env_vars, "CONTENT_TYPE", "X-CGI-Content-Type");
    req = add_header(
        req,
        &env_vars,
        "GATEWAY_INTERFACE",
        "X-CGI-Gateway-Interface",
    );
    req = add_header(req, &env_vars, "PATH_INFO", "X-CGI-Path-Info");
    req = add_header(req, &env_vars, "PATH_TRANSLATED", "X-CGI-Path-Translated");
    req = add_header(req, &env_vars, "QUERY_STRING", "X-CGI-Query-String");
    req = add_header(req, &env_vars, "REMOTE_ADDR", "X-CGI-Remote-Addr");
    req = add_header(req, &env_vars, "REMOTE_HOST", "X-CGI-Remote-Host");
    req = add_header(req, &env_vars, "REMOTE_IDENT", "X-CGI-Remote-Ident");
    req = add_header(req, &env_vars, "REMOTE_USER", "X-CGI-Remote-User");
    req = add_header(req, &env_vars, "REQUEST_METHOD", "X-CGI-Request-Method");
    req = add_header(req, &env_vars, "SCRIPT_NAME", "X-CGI-Script-Name");
    req = add_header(req, &env_vars, "SERVER_PORT", "X-CGI-Server-Port");
    req = add_header(req, &env_vars, "SERVER_PROTOCOL", "X-CGI-Server-Protocol");
    req = add_header(req, &env_vars, "SERVER_SOFTWARE", "X-CGI-Server-Software");

    req.body(stdin).unwrap()
}

// add the CGI request meta-variables as X-CGI- headers
fn add_header(
    req: http::request::Builder,
    env_vars: &HashMap<String, String>,
    meta_var: &str,
    target_header: &str,
) -> http::request::Builder {
    if let Some(var) = env_vars.get(meta_var) {
        req.header(target_header, var.as_str())
    } else {
        req
    }
}

/// Convert the Request into the appropriate stdout format
fn serialize_response(response: Response) -> Vec<u8> {
    let mut output = String::new();
    output.push_str("Status: ");
    output.push_str(response.status().as_str());
    if let Some(reason) = response.status().canonical_reason() {
        output.push_str(" ");
        output.push_str(reason);
    }
    output.push_str("\n");

    {
        let headers = response.headers();
        let mut keys: Vec<&http::header::HeaderName> = headers.keys().collect();
        keys.sort_by_key(|h| h.as_str());
        for key in keys {
            output.push_str(key.as_str());
            output.push_str(": ");
            output.push_str(headers.get(key).unwrap().to_str().unwrap());
            output.push_str("\n");
        }
    }

    output.push_str("\n");

    let mut output = output.into_bytes();

    let (_, mut body) = response.into_parts();

    output.append(&mut body);

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(input: Vec<(&str, &str)>) -> HashMap<String, String> {
        input
            .into_iter()
            .map(|(a, b)| (a.to_owned(), b.to_owned()))
            .collect()
    }

    #[test]
    fn test_empty() {
        let env_vars = env(vec![]);
        let stdin = Vec::new();
        let req = parse_request(env_vars, stdin);
        assert_eq!(req.method(), &http::method::Method::GET);
        // We don't want to assert any particular values for
        // anything else in the request, but as long as the above
        // didn't panic we're good.
    }

    #[test]
    fn test_parse_request() {
        let env_vars = env(vec![
            ("REQUEST_METHOD", "GET"),
            ("SCRIPT_NAME", "/my/path/script"),
            ("SERVER_PROTOCOL", "HTTP/1.0"),
            ("HTTP_USER_AGENT", "MyBrowser/1.0"),
            ("QUERY_STRING", "foo=bar&baz=bop"),
        ]);
        let stdin = Vec::new();
        let req = parse_request(env_vars, stdin);
        assert_eq!(req.method(), &http::method::Method::GET);
        assert_eq!(req.uri(), "/my/path/script?foo=bar&baz=bop");
        assert_eq!(req.uri().path(), "/my/path/script");
        assert_eq!(req.uri().query(), Some("foo=bar&baz=bop"));
        assert_eq!(req.version(), http::version::Version::HTTP_10);
        assert_eq!(req.headers()[http::header::USER_AGENT], "MyBrowser/1.0");
        assert_eq!(req.body(), &vec![] as &Vec<u8>);
    }

    fn test_serialized_response(resp: http::response::Builder, body: &str, expected_output: &str) {
        let resp: Response = resp.body(String::from(body).into_bytes()).unwrap();
        let output = serialize_response(resp);
        let expected_output = String::from(expected_output).into_bytes();

        if output != expected_output {
            println!(
                "output: {}\nexptected: {}",
                std::str::from_utf8(&output).unwrap(),
                std::str::from_utf8(&expected_output).unwrap()
            );
        }

        assert_eq!(output, expected_output);
    }

    #[test]
    fn test_serialized_response1() {
        test_serialized_response(
            http::Response::builder().status(200),
            "Hello World",
            "Status: 200 OK\n\nHello World",
        );

        test_serialized_response(
            http::Response::builder().status(200)
                .header("Content-Type", "text/html")
                .header("Content-Language", "en")
                .header("Cache-Control", "max-age=3600"),
            "<html><body><h1>Hello</h1></body></html>",
            "Status: 200 OK\ncache-control: max-age=3600\ncontent-language: en\ncontent-type: text/html\n\n<html><body><h1>Hello</h1></body></html>"
        );
    }

    #[test]
    fn test_shortcuts1() {
        assert_eq!(std::str::from_utf8(&serialize_response(html_response(200, "<html><body><h1>Hello World</h1></body></html>"))).unwrap(),
            "Status: 200 OK\ncontent-length: 46\ncontent-type: text/html; charset=utf-8\n\n<html><body><h1>Hello World</h1></body></html>"
        );
    }

    #[test]
    fn test_shortcuts2() {
        assert_eq!(
            std::str::from_utf8(&serialize_response(binary_response(
                200,
                None,
                vec![65, 66, 67]
            )))
            .unwrap(),
            "Status: 200 OK\ncontent-length: 3\n\nABC"
        );

        assert_eq!(
            std::str::from_utf8(&serialize_response(binary_response(
                200,
                "application/octet-stream",
                vec![65, 66, 67]
            )))
            .unwrap(),
            "Status: 200 OK\ncontent-length: 3\ncontent-type: application/octet-stream\n\nABC"
        );

        let ct: String = "image/png".to_string();
        assert_eq!(
            std::str::from_utf8(&serialize_response(binary_response(
                200,
                ct.as_str(),
                vec![65, 66, 67]
            )))
            .unwrap(),
            "Status: 200 OK\ncontent-length: 3\ncontent-type: image/png\n\nABC"
        );
    }

    #[test]
    fn test_handle_success() {
        let input = std::io::Cursor::new(vec![]);
        let mut output = std::io::BufWriter::new(Vec::new());
        let mut error = std::io::BufWriter::new(Vec::new());

        try_handle_with_io(
            |_req: Request| Ok::<http::Response<Vec<u8>>, String>(text_response(200, "All good")),
            input,
            &mut output,
            &mut error,
        );

        let written = output.into_inner().unwrap();
        assert_eq!(String::from_utf8(written).unwrap(), "Status: 200 OK\ncontent-length: 8\ncontent-type: text/plain; charset=utf-8\n\nAll good");
        assert_eq!(error.into_inner().unwrap().len(), 0);
    }

    #[test]
    fn test_handle_error() {
        let input = std::io::Cursor::new(vec![]);
        let mut output = std::io::BufWriter::new(Vec::new());
        let mut error = std::io::BufWriter::new(Vec::new());

        try_handle_with_io(
            |_req: Request| Err("Not good"),
            input,
            &mut output,
            &mut error,
        );

        let written = output.into_inner().unwrap();
        assert_eq!(
            String::from_utf8(written).unwrap(),
            "Status: 500 Internal Server Error\n\n"
        );
        assert_eq!(
            String::from_utf8(error.into_inner().unwrap()).unwrap(),
            "\"Not good\"\n"
        );
    }
}
