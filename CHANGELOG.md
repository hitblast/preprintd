## Changelog

Active since `v0.1.0`.

### v0.7.2

- Fix: nothing much, just some `clippy` errors.
- Reduced `derive` macro implementations in some structs.
- Added a new `BASE_URL` environment variable, which takes in the primary API URL the worker is going to use.
- Unmerged the large `sock!()` macro into a more concise `crate::socket::create_lpr_sock` method (in a new `crate::socket` module).
- LPR connectivity timeout for the `crate::is_online` function has been set to 1 second (from 800 milliseconds).
- Added a new `consts` module (to be improved later).

### v0.7.1

- `ALIAS` now defaults to the `CARGO_PKG_NAME` environment variable specified during compile-time.
- Fix a potential missing-index bug in `crate::doh::ready_tls_config`.
- Added a proper DoH cache with expiry, minimum-TTL handing, and some other nitty-gritty features.
- Removed dynamic dispatch and `serde_json::Value` usage from the project (now I hate calling it a "codebase"), opting for statically-typed serialization and deserialization.
- Minor code readability improvements.

### v0.7.0

- The preconfigured TLS backend with ECH is now optional (if unable to be resolved).
- Implemented a proper DoH (DNS over HTTPS) resolver struct (`DohResolver`) using the `reqwest::dns::Resolve` trait.
- Internal adjustments include reducing the dependency on `serde_json::Value` and using static types for deserialization instead.

### v0.6.5

- Removed the JWT authorization header from the request flow.

### v0.6.4

- Removed pinger thread.
- The `SO_LINGER` option is no longer set if the printer socket errors out while sending a job.
- Regardless of error events in the printer socket connection, the `abortive` and `transferred` variables has been removed, and the socket is always shut down gracefully.

### v0.6.3

- `User-Agent` header now outputs the package version in the latter part of it.
- Printer stream-read errors are now slighly more elaborate (for testing purposes).
- `crate::client::build_client_or_default()` has been removed.
- Following the change above, the functions in the `client` module have been modified to propagate client-creation errors upto the main function to retry with backoff instead of panicking.
- The clients used for the primary network requests are now refreshed 250 seconds after their creation (previously 300).

### v0.6.2

- Fixed a silly but critical bug with reading the identity file caused in v0.6.1.

### v0.6.1

- Removed `Last-Event-ID` header implementation.
- `STATE_DIRECTORY` is now auto-created if it is passed in as an environment variable but does not exist.
- Improved path management for the `.ident` file inside the `crate::ident::decide_ident` function.
- Removed internal `CLAIM_COUNT` delay - true worker race has been unleashed again!
- Removed `.timeout()` while building the HTTP client inside `crate::client::build_client`, and instead replaced it with a `.connect_timeout()`.
- Following the change mentioned immediately above, request timeouts have been adjusted for all `reqwest::blocking::Client` instances inside the codebase.

### v0.6.0

- Zeroed panic-sites:
  - In `sock!()`, the DNS resolution is now offloaded to a new thread and waited upon with a timeout instead of blocking forever.
  - The `sock!()` macro now also uses a saturating duration to calculate remaining retries.
- An explicit 30-second timeout has been added to the `crate::client::build_client` function.
- `crate::client::CLIENT` now uses `build_client_or_default` as its builder-function, which internally retries for 10 times (backoff included) before panicking unavoidably.
- Improved client-rebuild timing logic in `crate::client::client`.

### v0.5.2

- Fixed compilation errors caused by the `serde` dependency (was missing the `derive` feature).
- `zbus` is now a Linux-only dependency.

### v0.5.1

- Removed unused `rand` method and `encrypt` function from the `crypto` module.
- Upgraded `uuid` and `zbus` dependencies.

### v0.5.0

- Added experimental support for the ECH Protocol for building the API client using `rustls`.

### v0.4.1

- Fix `clippy`-suggested issues.

### v0.4.0

- Calling `debug_log!()` no longer requires having `crate::DEBUG` in the same scope.
- Identity is now initialized using the `crate::ident::decide_ident` function alone.
- `crate::ident::create_new_ident` is now private.
- Used bare HTTP SSE instead of Mercure.

### v0.3.11

- Placed the worker rest logic under `handle()`.

### v0.3.10

- Fixed a bug which led to the worker ping thread not waking up or reconnecting after a ping failure.

### v0.3.9

- Improvements have been made to how the worker identity is decided per session.
- Removed the `IS_PRINT_PROCESSING` atomic boolean and replaced it with a `PRINT_LOCK` mutex.
- Added a slight delay after completing at least 3 jobs to give chance to other, concurrently running workers.

### v0.3.8

- Attempting to fix repeated Mercure stream errors and socket drops.
- Fixed scope issue related to the inhibitor lock preventing sleep.

### v0.3.7

- Set default target for Linux builds to: `x86_64-unknown-linux-musl`

### v0.3.6

- Add proper `std::fs` read-write calls to introduce proper state directory functionality.

### v0.3.5

- `claim_job` now receives `job_id` as `&str`, and `handle` now make sures to not take jobs with empty IDs.
- `debug_log!()` now prints messages with `LogLevel::Ok` with `println!()`.

### v0.3.4

- Removed `X-Worker-Spooler`, since we do not need it anymore from the server-level.
- Removed encryption for `X-Worker-Ident`, it now passes a raw UUID (v4) every time a new worker instance boots up.
- Added support for inhibitor FD locks using the `--inhibit` flag to disable sleeping on Linux machines while the program is running. The `org.freedesktop.login1*` D-Bus interfaces are used for this function.
- Minor code optimizations have been done.

### v0.3.3

- Add support for the `X-Worker-Spooler` header again, which tells the server a job is ongoing for any given worker instance.

### v0.3.2

- Working version; reverted some `LazyLock` shenanigans back to the old version.

### v0.3.1

- Attempt to fix issues with coercion, leading to the "WORKER_KEY is unauthorized" bug.

### v0.3.0

- `X-Worker-Ident` can no longer be disabled.
- `X-Worker-Ident` is now only passed into requests during a claim-job attempt.
- `X-Worker-Ident` is now encrypted and then Base64-encoded before being passed into HTTP requests.
- `X-Worker-Ident` is now static (generated once and stored in the state directory).
- Moved client-creation helpers into its own `client` module.
- Added a new `encrypt()` function in the `crypto` module.

### v0.2.6

- The `hdrs()` function now passes in a new `X-Worker-Ident` header with the `HeaderMap` it generates.
- Fixed the debug log with mercure endpoint.

### v0.2.5

- Removed unnecessary `LazyLock` from variables inside the `consts` (previously `constant`) module.
- `decrypt()` function now uses the `WORKER_KEY` constant from the global scope and not from its parameters.

### v0.2.4

- Enhanced real-time stream handling and logging.
- Detached `AGENT`, `DEF_HOST` and `DEF_QUEUE` from source and used `LazyLock` instances like `WORKER_KEY` to pull them from the environment during runtime instead.

### v0.2.3

(experimental release; no changelog provided during the time of testing)

### v0.2.2

- Changed the `stream()` function so that the `handle()` call is moved to a separate thread.

### v0.2.1

- Removed `X-Worker-Spooler` implementation from the `hdrs()` function.
- Changed signatures of the `hdrs()` and `claim_job()` functions following the change above.

### v0.2.0

- Complete feature parity referencing the Python implementation.

### v0.1.4

- Better `debug_log!()` placement across the codebase.

### v0.1.3

- Experimental changes to socket (more `Option<String>` fields in `crate::types::Job`).

### v0.1.2

- Change write/recv sequence (experimental).

### v0.1.1

- Add support for the `--debug` flag.

### v0.1.0

- Initial launch.
