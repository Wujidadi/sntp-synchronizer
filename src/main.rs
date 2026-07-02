use std::net::{ToSocketAddrs, UdpSocket};
use std::path::PathBuf;
use std::time::Duration;

use chrono::{Local, TimeZone, Utc};

// Built-in default list of SNTP servers, tried from top to bottom by priority
// Used as a fallback when no config file is found or its contents are empty
const SERVERS: &[&str] = &[
    "time.stdtime.gov.tw",
    "tock.stdtime.gov.tw",
    "watch.stdtime.gov.tw",
    "clock.stdtime.gov.tw",
    "tick.stdtime.gov.tw",
    "time.google.com",
    "time.asia.apple.com",
    "time.apple.com",
    "time.euro.apple.com",
    "time.cloudflare.com",
    "time.windows.com",
    "time1.facebook.com",
    "time2.facebook.com",
    "time3.facebook.com",
    "time4.facebook.com",
    "time5.facebook.com",
];

// Config file name, used both in the working directory and the user config directory
// One host name per line; lines starting with # are comments and blank lines are ignored
const SERVERS_FILE: &str = "servers.conf";

// Environment variable used to override the config file path
const SERVERS_FILE_ENV: &str = "SNTP_SERVERS_FILE";

// Subdirectory name under the user config directory, i.e. ~/.config/<this name>/servers.conf
const CONFIG_DIR_NAME: &str = "sntp-synchronizer";

// NTP port
const NTP_PORT: u16 = 123;

// Seconds between 1900-01-01 and 1970-01-01, used to convert NTP time to Unix time
const NTP_UNIX_DELTA: u64 = 2_208_988_800;

// Timeout in seconds for a single query
const TIMEOUT_SECS: u64 = 3;

// ANSI color codes
const GREEN: &str = "\x1b[92m"; // bright green
const BLUE: &str = "\x1b[94m"; // blue
const RED: &str = "\x1b[91m"; // red
const RESET: &str = "\x1b[0m";

// Return the config file candidate paths in order: environment variable, working directory, user config directory
fn config_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // 1. Path given by the environment variable (highest priority)
    if let Ok(p) = std::env::var(SERVERS_FILE_ENV) {
        if !p.is_empty() {
            paths.push(PathBuf::from(p));
        }
    }

    // 2. Config file in the working directory (convenient for development and in-project runs)
    paths.push(PathBuf::from(SERVERS_FILE));

    // 3. User config directory (standard location after a global install)
    if let Some(dir) = user_config_dir() {
        paths.push(dir.join(CONFIG_DIR_NAME).join(SERVERS_FILE));
    }

    paths
}

// Resolve the user config directory, preferring XDG_CONFIG_HOME and falling back to ~/.config
fn user_config_dir() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return Some(PathBuf::from(xdg));
        }
    }

    std::env::var("HOME")
        .ok()
        .filter(|home| !home.is_empty())
        .map(|home| PathBuf::from(home).join(".config"))
}

// Parse the config file contents into a list of host names, stripping per-line comments and surrounding whitespace and dropping blank lines
fn parse_servers(content: &str) -> Vec<String> {
    content
        .lines()
        .map(|line| line.split('#').next().unwrap_or("").trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

// Load the server list by reading the candidate paths in order, falling back to the built-in default when none yield a list
fn load_servers() -> Vec<String> {
    for path in config_candidates() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            let list = parse_servers(&content);
            if !list.is_empty() {
                return list;
            }
        }
    }

    // Fall back to the built-in default list
    SERVERS.iter().map(|s| s.to_string()).collect()
}

// Query a single SNTP server, returning (Unix seconds, nanoseconds) on success
fn query(server: &str) -> Option<(i64, u32)> {
    // Resolve the host name and take the first usable address
    let addr = (server, NTP_PORT).to_socket_addrs().ok()?.next()?;

    // Bind to an arbitrary local port and set the send/receive timeouts
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    let timeout = Some(Duration::from_secs(TIMEOUT_SECS));
    socket.set_read_timeout(timeout).ok()?;
    socket.set_write_timeout(timeout).ok()?;
    socket.connect(addr).ok()?;

    // Build the 48-byte NTP request packet; the first byte 0x1B means LI=0, VN=3, Mode=3 (client)
    let mut request = [0u8; 48];
    request[0] = 0x1B;
    socket.send(&request).ok()?;

    // Receive the response and check its length
    let mut response = [0u8; 48];
    let received = socket.recv(&mut response).ok()?;
    if received < 48 {
        return None;
    }

    // The Transmit Timestamp is at offset 40: the first 4 bytes are the seconds and the last 4 bytes are the fraction
    let seconds = u32::from_be_bytes([response[40], response[41], response[42], response[43]]) as u64;
    let fraction = u32::from_be_bytes([response[44], response[45], response[46], response[47]]) as u64;

    // Treat a zero seconds value as an invalid response (Kiss-o'-Death or a malformed packet)
    if seconds == 0 {
        return None;
    }

    let unix_secs = seconds.checked_sub(NTP_UNIX_DELTA)? as i64;
    let nanos = (fraction * 1_000_000_000 >> 32) as u32;

    Some((unix_secs, nanos))
}

fn main() {
    for server in load_servers() {
        if let Some((secs, nanos)) = query(&server) {
            // Convert to the local time zone and format the output
            if let Some(utc) = Utc.timestamp_opt(secs, nanos).single() {
                let local = utc.with_timezone(&Local);
                let formatted = local.format("%Y-%m-%d %H:%M:%S%.3f %z").to_string();

                println!("{GREEN}{formatted}{RESET}");
                println!("{BLUE}Source SNTP server: {server}{RESET}");
                return;
            }
        }
    }

    // A full round of polling still yielded no result
    eprintln!("{RED}SNTP time synchronization failed{RESET}");
    std::process::exit(1);
}
