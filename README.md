## preprintd

Printer swarm-worker daemon implementation for [PreConnect](https://github.com/sabbirba/preconnect).

> [!NOTE]
> Development happens on the [Codeberg repository](https://codeberg.org/hitblast/preprintd), and the [GitHub repository](https://github.com/hitblast/preprintd) is only for receiving pull requests and pushing prebuilt binaries along with the releases.

### Overview

This tiny worker is just a stateless SSE/`TcpStream` hybrid under the hood, which constantly listens for incoming jobs from the printer SSE endpoint of the [PreConnect API](https://github.com/sabbirba/preconnect), and dispatches them over to the on-campus LPR server via a simple `TcpStream`.

### Compiling

Requires [Rust](https://rust-lang.org/) (2024 edition or later) to be installed.

Run the traditional release command:

```bash
cargo build --release
```

You can also directly install the `preprintd` binary globally using [cargo](https://github.com/rust-lang/cargo):

```bash
cargo install preprintd
```

> [!NOTE]
> The release binary is optimized for the smallest-possible size, although you can change this behavior by disabling the optimizations specified in the `[profile.release]` section of [Cargo.toml](./Cargo.toml).

### Prebuilt Binaries

See the [GitHub Releases](https://github.com/hitblast/preprintd/releases) for a prebuilt binary for either Windows, Linux (built via CI workers running Ubuntu), or macOS. The Linux builds are done using the `x86_64-unknown-linux-musl` target in Rust (see [musl.cc](https://musl.cc/)).

### Daemon Usage

Create a new `systemd` service which you can enable later:

```bash
sudo touch /etc/systemd/system/preprintd.service

# or, if you want to run it as a user unit:
# touch ~/.config/systemd/user/preprintd.service
```

Then, write [this INI configuration](./preprintd.service) in your `preprintd.service` file.

Make sure to replace the following fields/values:

1. Under `Environment=`:

- (Optional) `BASE_URL`: The primary API URL that the worker is going to communicate with.
- (Optional) `ALIAS`: The name which determines the program's identity on the system and in TCP requests.
- `WORKER_KEY`: Your worker key credential (from the PreConnect API).
- `DEF_HOST`: The default printer host to use in case the API cannot provide one.
- `DEF_QUEUE`: The default queue name to send printable data to.

2. Replace `/usr/bin/preprintd` with the appropriate path to the daemon binary.
3. Replace `username` with your appropriate username on the machine. **Note that this step is important if you want to use `--inhibit` later on (see below).**

> [!WARNING]
> If you prefer to use the service as a **user unit** (or, in other words, by passing in `systemctl --user`), please make sure to omit the `User` field from `[Service]`, otherwise it may cause errors during service startup.

Once you are done, enable and start the service:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now preprintd

# or, for running as a user unit:
# systemctl --user daemon-reload
# systemctl --user enable --now preprintd

# now check status:
systemctl status preprintd
```

To check the logs in real-time, run:

```bash
journalctl -u preprintd -f
```

#### Inhibitor Locks (Linux-only)

You may pass in the `--inhibit` flag while running `preprintd` to acquire an inhibitor FD (or "file descriptor") for the lifecycle of the program. This will prevent your Linux machine from sleeping. This may be crucial if you computer auto-suspends, and suspension may kill outward connections such as the running HTTP SSE from the program.

The file descriptor is derived from the `org.freedesktop.login1` service. While running the daemon with `--inhibit`, you can test out the functionality by running `systemd-inhibit --list` and checking if `preprintd` pops up anywhere.

> [!NOTE]
> Using inhibitor locks may not prevent manual suspension/sleep.

### Code Inspection

Some things you might come across while navigating this project are noted down below:

- The standard LPR/LPD sequence is present for sending requests to the LPR server. Note that the connections are **one-time**, meaning that once a job has been dispatched successfully, the socket connection is dropped. It is recreated once another job has been received.
- Lots of `LazyLock` usage is present. Although this is not optimal for a program that's supposed to be tiny, this pattern has been used to reuse as much data as physically possible to (slightly) prevent hardcoding and messing things up.
- The HTTP requests are made using a `reqwest::blocking::Client` instance, and in general there is zero async I/O inside the codebase. Some tasks are just offloaded to separate threads (i.e. decoupling a `Job` using `handle()`).
- The `client` module provides:
  - a `ready_tls_config()` function which creates a custom TLS backend configuration for the `reqwest::blocking::Client` instance to enable TLS 1.3 and [ECH (Encrypted Client Hello)](https://blog.cloudflare.com/announcing-encrypted-client-hello/) support.
  - a custom DNS resolver implementation via the `DohResolver` struct (which uses [DoH (DNS over HTTPS)](https://developers.cloudflare.com/1.1.1.1/encryption/dns-over-https/) under the hood and implements `reqwest::dns::Resolve`), which is used for long-haul connections to the PreConnect API SSE.

More specific parts of the codebase that you may be more curious about are described below:

#### Windows Inconsistencies

Although most of the instructions above are primarily made for Linux (and can be migrated over to Unix/macOS), some built-in features are not available on the Windows operating system by default. For example, the `STATE_DIRECTORY` environment variable set via `systemd` during runtime never shows up there. Moreover, some Windows-specific features might be missing from this implementation entirely, for which it is encouraged that you give the [Reference Implementation](#reference-implementation) a try.

#### Identifying Workers

While claiming a job, each worker identifies itself with an `X-Worker-Ident` header which has a pattern of `<UUID>;<ARCH>` (e.g. `03780793-e7af-49c1-b55d-92ff57be8c6e;aarch64-apple-darwin`).

Visible from the pattern, the identity has two parts:

- `<UUID>`: A randomly-generated UUID (v4) string literal, which is used to give the worker a unique identity to be correlated with.
- `<ARCH>`: The architecture of _the compiled binary_ of the worker.

#### Worker Health-Check

Every worker can occasionally become prone to errors due to software glitches, hence another header, namely `X-Worker-Health`, is passed in with all the requests that the worker makes.

### Reference Implementation

See: https://github.com/sabbirba/preconnect/blob/main/printer.py (courtesy: [@sabbirba](https://github.com/sabbirba))

### License

Licensed under the [GNU General Public License v3](./LICENSE).
