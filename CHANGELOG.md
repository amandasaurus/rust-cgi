0.6.1 (2024-01-20)
------------------

 * Some README updates
 * Repo forked from amandasaurus/rust-cgi to staktrace/rust-cgi


> [!NOTE]
> Old changelog entries below refer to the `cgi` crate (prior to the `rust-cgi` fork)

0.6 (2020-05-27)
----------------

 * Correctly set charset to utf8 for HTML responses (thanks Unrealrussian)
 * `binary_response` can now optionally set a `Content-Type` header.
 * Main handler function now accepts `FnOnce`

0.5 (2020-04-11)
----------------

 * Upgrade `http` dependency from 0.1 to 0.2

0.4 (2020-04-11)
----------------

 * Add `text_response` for plain text response (v. similar to `string_response`)
 * Add `cgi_main!` and `cgi_try_main!` macros to reduce boilerplate

0.3.1 (2019-10-05)
----------------

 * Correctly publish

0.3.0 (2019-10-05) ''yanked''
----------------

 * Basic support for HTTP/2

0.2.0 (2018-02-18
----------------

 * Improved tests
