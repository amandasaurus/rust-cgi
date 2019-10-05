//! Easily create CGI (RFC 3875) programmes in Rust based on [`http`](https://github.com/hyperium/http)
//! 
//! # Installation & Usage
//! 
//! `Cargo.toml`:
//! 
//! ```cargo,ignore
//! [dependencies]
//! cgi = "0.1"
//! ```
//! 
//! In the `main` function, call only `cgi::handle(...)`, with a function that
//! takes a `cgi::Request` and returns `cgi::Response`.
//! 
//! ```rust,ignore
//! extern crate cgi;
//! 
//! fn main() { cgi::handle(|request: cgi::Request| -> cgi::Response {
//!      // ...
//! })}
//! ```
//! 
//! [Hello World](https://en.wikipedia.org/wiki/%22Hello,_World!%22_program):
//! 
//! ```rust,ignore
//! extern crate cgi;
//! 
//! fn main() { cgi::handle(|request: cgi::Request| -> cgi::Response {
//!     cgi::html_response(200, "<html><body><h1>Hello World!</h1></body></html>")
//! })}
//! ```
//! 
//! It will parse & extract the CGI environmental variables, and HTTP request body
//! to create `Request`, and convert your `Response` into the correct format and
//! print to stdout. If this programme is not called as CGI (e.g. missing required
//! environmental variables), it will panic.


use std::io::{Read, Write, stdin};
use std::collections::HashMap;

pub extern crate http;

/// A `Vec<u8>` Request from http
pub type Request = http::Request<Vec<u8>>;

/// A `Vec<u8>` Response from http
pub type Response = http::Response<Vec<u8>>;

/// Call F as a CGI programme.
///
/// Parse & extract the CGI environmental variables, and HTTP request body
/// to create `Request`, and convert your `Response` into the correct format and
/// print to stdout. If this programme is not called as CGI (e.g. missing required
/// environmental variables), it will panic.
pub fn handle<F>(func: F) 
    where F: Fn(Request) -> Response
{
    let env_vars: HashMap<String, String> = std::env::vars().collect();

    // How many bytes do we have to read for request body
    // A general stdin().read_to_end() can block if the webserver doesn't close things
    let content_length: usize = env_vars.get("CONTENT_LENGTH").and_then(|cl| cl.parse::<usize>().ok()).unwrap_or(0);
    let mut stdin_contents = vec![0; content_length];
    stdin().read_exact(&mut stdin_contents).unwrap();

    let request = parse_request(env_vars, stdin_contents);

    let response = func(request);

    let output = serialize_response(response);

    std::io::stdout().write_all(&output).unwrap();
}

/// A HTTP Reponse with no body and that HTTP status code, e.g. `return cgi::empty_response(404);`
/// to return a [HTTP 404 Not Found](https://en.wikipedia.org/wiki/HTTP_404).
pub fn empty_response<T>(status_code: T) -> Response
    where http::StatusCode: http::HttpTryFrom<T>
{
    http::response::Builder::new().status(status_code).body(vec![]).unwrap()
}

/// Converts `text` to bytes (UTF8) and sends that as the body with that `status_code` and HTML
/// `Content-Type` header.
pub fn html_response<T, S>(status_code: T, body: S) -> Response
    where http::StatusCode: http::HttpTryFrom<T>,
          S: Into<String>
{
    let body: Vec<u8> = body.into().into_bytes();
    http::response::Builder::new()
        .status(status_code)
        .header(http::header::CONTENT_TYPE, "text/html")
        .header(http::header::CONTENT_LENGTH, format!("{}", body.len()).as_str())
        .body(body)
        .unwrap()
}

/// Returns a simple plain text response.
pub fn string_response<T, S>(status_code: T, body: S) -> Response
    where http::StatusCode: http::HttpTryFrom<T>,
          S: Into<String>
{
    let body: Vec<u8> = body.into().into_bytes();
    http::response::Builder::new()
        .status(status_code)
        .header(http::header::CONTENT_LENGTH, format!("{}", body.len()).as_str())
        .body(body)
        .unwrap()
}

/// Sends  `blob` with that status code.
pub fn binary_response<T>(status_code: T, body: Vec<u8>) -> Response
    where http::StatusCode: http::HttpTryFrom<T>
{
    http::response::Builder::new()
        .status(status_code)
        .header(http::header::CONTENT_LENGTH, format!("{}", body.len()).as_str())
        .body(body)
        .unwrap()
}


fn parse_request(env_vars: HashMap<String, String>, stdin: Vec<u8>) -> Request {
    let mut req = http::Request::builder();

    req.method(env_vars["REQUEST_METHOD"].as_str());
    let uri = if env_vars.get("QUERY_STRING").unwrap_or(&"".to_owned()) != "" {
        format!("{}?{}", env_vars["SCRIPT_NAME"], env_vars["QUERY_STRING"])
    } else {
        env_vars["SCRIPT_NAME"].to_owned()
    };
    req.uri(uri.as_str());

    if let Some(v) = env_vars.get("SERVER_PROTOCOL") {
        if v == "HTTP/0.9" {
            req.version(http::version::Version::HTTP_09);
        } else if v == "HTTP/1.0" {
            req.version(http::version::Version::HTTP_10);
        } else if v == "HTTP/1.1" {
            req.version(http::version::Version::HTTP_11);
        } else if v == "HTTP/2.0" {
            req.version(http::version::Version::HTTP_2);
        } else {
            unimplemented!("Unsupport HTTP SERVER_PROTOCOL {:?}", v);
        }
    }

    for key in env_vars.keys().filter(|k| k.starts_with("HTTP_")) {;
        let header: String = key.chars().skip(5).map(|c| if c == '_' { '-' } else { c }).collect();
        req.header(header.as_str(), env_vars[key].as_str().trim());
    }


    add_header(&mut req, &env_vars, "AUTH_TYPE", "X-CGI-Auth-Type");
    add_header(&mut req, &env_vars, "CONTENT_LENGTH", "X-CGI-Content-Length");
    add_header(&mut req, &env_vars, "CONTENT_TYPE", "X-CGI-Content-Type");
    add_header(&mut req, &env_vars, "GATEWAY_INTERFACE", "X-CGI-Gateway-Interface");
    add_header(&mut req, &env_vars, "PATH_INFO", "X-CGI-Path-Info");
    add_header(&mut req, &env_vars, "PATH_TRANSLATED", "X-CGI-Path-Translated");
    add_header(&mut req, &env_vars, "QUERY_STRING", "X-CGI-Query-String");
    add_header(&mut req, &env_vars, "REMOTE_ADDR", "X-CGI-Remote-Addr");
    add_header(&mut req, &env_vars, "REMOTE_HOST", "X-CGI-Remote-Host");
    add_header(&mut req, &env_vars, "REMOTE_IDENT", "X-CGI-Remote-Ident");
    add_header(&mut req, &env_vars, "REMOTE_USER", "X-CGI-Remote-User");
    add_header(&mut req, &env_vars, "REQUEST_METHOD", "X-CGI-Request-Method");
    add_header(&mut req, &env_vars, "SCRIPT_NAME", "X-CGI-Script-Name");
    add_header(&mut req, &env_vars, "SERVER_PORT", "X-CGI-Server-Port");
    add_header(&mut req, &env_vars, "SERVER_PROTOCOL", "X-CGI-Server-Protocol");
    add_header(&mut req, &env_vars, "SERVER_SOFTWARE", "X-CGI-Server-Software");

    req.body(stdin).unwrap()
    
}

// add the CGI request meta-variables as X-CGI- headers
fn add_header(req: &mut http::request::Builder, env_vars: &HashMap<String, String>, meta_var: &str, target_header: &str) {
    if let Some(var) = env_vars.get(meta_var) {
        req.header(target_header, var.as_str());
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
        input.into_iter().map(|(a, b)| (a.to_owned(), b.to_owned())).collect()
    }

    #[test]
    fn test_parse_request() {
        let env_vars = env(vec![
           ("REQUEST_METHOD", "GET"), ("SCRIPT_NAME", "/my/path/script"),
           ("SERVER_PROTOCOL", "HTTP/1.0"), ("HTTP_USER_AGENT", "MyBrowser/1.0"),
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
        assert_eq!(req.body(), &vec![]);
    }

    fn test_serialized_response(resp: &mut http::response::Builder, body: &str, expected_output: &str) {
        let resp: Response = resp.body(String::from(body).into_bytes()).unwrap();
        let output = serialize_response(resp);
        let expected_output = String::from(expected_output).into_bytes();

        if output != expected_output {
            println!("output: {}\nexptected: {}", std::str::from_utf8(&output).unwrap(), std::str::from_utf8(&expected_output).unwrap());
        }

        assert_eq!(output, expected_output);
    }

    #[test]
    fn test_serialized_response1() {
        test_serialized_response(
            http::Response::builder().status(200),
            "Hello World",
            "Status: 200 OK\n\nHello World"
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
}
