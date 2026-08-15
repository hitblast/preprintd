## `preprintd`

Printer swarm-worker daemon implementation for [PreConnect](https://github.com/sabbirba/preconnect).

[(Codeberg Mirror)](https://codeberg.org/hitblast/preprintd)

### Overview

This tiny worker is just a `TcpStream` under the hood, constantly listening for jobs and claiming one if open. It works by constantly listening for incoming data from the `api.preconnect.app` endpoint (which uses an HTTP SSE connection under the hood for streaming real-time data), and then initiating the claiming procedure.

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
```

Write [this INI configuration](./preprintd.service) in your `preprintd.service` file. Make sure to replace the following fields/values:

1. Under `Environment=`:

- `WORKER_KEY`: Your worker key credential (from the PreConnect API).
- `DEF_HOST`: The default printer host to use in case the API cannot provide one.
- `DEF_QUEUE`: The default queue name to send printable data to.
- (Optional) `ALIAS`: The name which determines the program's identity on the system and in TCP requests.

2. Replace `/usr/bin/preprintd` with the appropriate path to the daemon binary.
3. Replace `username` with your appropriate username on the machine. **Note that this step is important if you want to use `--inhibit` later on (see below).**

Once you are done, enable and start the service:

```bash
sudo systemctl daemon-reload
sudo systemctl enable preprintd.service
sudo systemctl start preprintd.service

# now check status:
systemctl status preprintd.service
```

> [!WARNING]
> If you prefer to use the service as a **user unit** (or, in other words, by passing in `systemctl --user`), please make sure to omit the `User` field from `[Service]`.

To check the logs in real-time, run:

```bash
journalctl -u preprintd.service -f
```

#### Inhibitor Locks (Linux-only)

You may pass in the `--inhibit` flag while running `preprintd` to acquire an inhibitor FD (or "file descriptor") for the lifecycle of the program. This will prevent your Linux machine from sleeping. This may be crucial if you computer auto-suspends, and suspension may kill outward connections such as the running HTTP SSE from the program.

The file descriptor is derived from the `org.freedesktop.login1` service. While running the daemon with `--inhibit`, you can test out the functionality by running `systemd-inhibit --list` and checking if `preprintd` pops up anywhere.

> [!NOTE]
> Using inhibitor locks may not prevent manual suspension/sleep.


### Code Inspection

When you're going through the code, you'll see these:

- The standard LPR/LPD sequence (except the code doing HTTP requests via [reqwest's](https://github.com/seanmonstar/reqwest) blocking API and every other code surrounding/using this logic).
- Lots of `LazyLock` usage. Although this is not optimal for a program that's supposed to be tiny, we've kept this pattern to reuse as much data as physically possible without hardcoding and messing up.

More specific parts of the codebase that you may be more curious about are described below:

#### Windows Inconsistencies

Although most of the instructions above are primarily made for Linux (and can be migrated over to Unix/macOS), some built-in features are not available on the Windows operating system by default. For example, the `STATE_DIRECTORY` environment variable set via `systemd` during runtime never shows up there. Moreover, some Windows-specific features might be missing from this implementation entirely, for which it is encouraged that you give the [Reference Implementation](#reference-implementation) a try.

#### Identifying Workers

While claiming a job, each worker identifies itself with an `X-Worker-Ident` header which has a pattern of `<UUID>;<ARCH>;<IDENTITY_TYPE>` (e.g. `03780793-e7af-49c1-b55d-92ff57be8c6e;aarch64-apple-darwin;static`).

Visible from the pattern mentioned above, the worker identity can be broken down into three parts:

- `<UUID>`: A randomly-generated UUID (v4) string literal, which is used to give the worker a unique identity to be correlated with.
- `<ARCH>`: The architecture of _the compiled binary_ of the worker.
- `<IDENTITY_TYPE>`: Another string literal representing whether the identity is **static** (when it successfully retrieves a previous identity or creates a new one under `$STATE_DIRECTORY/.ident` and _then_ retrieves it), or **dynamic** (due to issues with `std::fs` operations or just the `$STATE_DIRECTORY` path being unavailable).

### Reference Implementation

See: https://github.com/sabbirba/preconnect/blob/main/printer.py (courtesy: [@sabbirba](https://github.com/sabbirba))

### License

Licensed under the [GNU General Public License v3](./LICENSE).
