# sntp-synchronizer

A Rust command-line tool that polls multiple SNTP servers in priority order and synchronizes the system clock.

## Features

- Queries the servers one by one from top to bottom, stopping at the first valid result
- Applies the standard NTP offset formula to cancel out one-way network delay, then writes the corrected time to the system clock
- On success, prints the synchronized time in **bright green** and the source server in **blue**
- Timeouts, parse failures, and server errors are silently skipped, moving straight on to the next server
- Only after a full round of polling fails does it print `SNTP time synchronization failed` in **red** and exit with code 1

> **Note:** Setting the system clock requires root privileges, so run the tool with `sudo`. Without them it prints a hint to re-run with `sudo` and exits with code 1.

## Quick install

One command to "create the config file -> link it into the user config directory -> install the global executable":

```sh
./install
```

The script performs these steps in order:

1. Copies `servers.example.conf` to create `servers.conf` if no local one exists
2. Creates the symlink `~/.config/sntp-synchronizer/servers.conf` pointing to the local `servers.conf`
3. Runs `cargo install --path .` to install into `~/.cargo/bin`

Afterwards `sntp-synchronizer` can be run from any directory and will read the same config file through that link.

## Build and run

```sh
cargo build --release    # produce an optimized executable under target/release/
sudo cargo run           # run in development mode (root is required to set the clock)
cargo install --path .   # install only the global executable into ~/.cargo/bin (no config link)
```

Once installed, run it directly by name:

```sh
sudo sntp-synchronizer
```

## Output example

```
2026-07-01 18:53:38.534 +0800
Source SNTP server: time.stdtime.gov.tw
```

## Server list configuration

The server list can be adjusted through a config file. The config file is a local file that is not version-controlled; only the example file `servers.example.conf` is kept in version control.
When not using the quick install, create the config file from the example and adjust it as needed:

```sh
cp servers.example.conf servers.conf
```

The program looks for the config file in the following order, taking the first one that exists and is non-empty, and falling back to the built-in default list if none apply:

1. The path given by the `SNTP_SERVERS_FILE` environment variable
2. `./servers.conf` in the working directory (convenient for development and in-project runs)
3. `~/.config/sntp-synchronizer/servers.conf` (the standard location for a global install; may be a symlink, and honors `XDG_CONFIG_HOME`)
4. The built-in default list (the `SERVERS` constant in `src/main.rs`)

Config file format: one host name per line, tried from top to bottom by priority; lines starting with `#` are comments and blank lines are ignored.

```sh
SNTP_SERVERS_FILE=/path/to/my-servers.conf sntp-synchronizer
```

## Implementation notes

- Builds the 48-byte NTP request packet by hand with the std `UdpSocket` (first byte `0x1B` = LI 0 / VN 3 / Mode 3) and reads the server Receive and Transmit timestamps
- Computes the clock offset as `((T2 - T1) + (T3 - T4)) / 2` from the four NTP timestamps and writes the corrected time via `libc::settimeofday`
- Uses `chrono` for time conversion and time-zone formatting; colors are handled directly with ANSI codes

## Tunable parameters

Collected in the constants at the top of `src/main.rs`:

| Constant       | Description                                     |
| -------------- | ----------------------------------------------- |
| `SERVERS`      | Built-in default list (config fallback)         |
| `SERVERS_FILE` | Default config file path                        |
| `TIMEOUT_SECS` | Per-server query timeout in seconds (default 3) |
| `GREEN` etc.   | Colors for each status                          |
