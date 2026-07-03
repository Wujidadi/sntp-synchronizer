use std::net::{ToSocketAddrs, UdpSocket};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

// Read the local clock as a floating-point count of seconds since the Unix epoch
fn now_unix() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

// Convert an 8-byte NTP timestamp (32-bit seconds + 32-bit fraction) into seconds since the Unix epoch
fn ntp_to_unix(bytes: &[u8]) -> f64 {
    let seconds = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64;
    let fraction = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as f64;
    seconds - NTP_UNIX_DELTA as f64 + fraction / 4_294_967_296.0
}

// Query a single SNTP server, returning the corrected current Unix time (in seconds) on success
// The correction applies the standard NTP offset formula to cancel out one-way network delay
fn query(server: &str) -> Option<f64> {
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

    // Record the local transmit time (T1) right before sending and the local receive time (T4) right after receiving
    let t1 = now_unix();
    socket.send(&request).ok()?;

    // Receive the response and check its length
    let mut response = [0u8; 48];
    let received = socket.recv(&mut response).ok()?;
    let t4 = now_unix();
    if received < 48 {
        return None;
    }

    // Treat a zero Transmit Timestamp as an invalid response (Kiss-o'-Death or a malformed packet)
    if response[40..44] == [0, 0, 0, 0] {
        return None;
    }

    // Server Receive Timestamp (T2) at offset 32 and Transmit Timestamp (T3) at offset 40
    let t2 = ntp_to_unix(&response[32..40]);
    let t3 = ntp_to_unix(&response[40..48]);

    // Clock offset between this host and the server, averaging out the round-trip delay
    let offset = ((t2 - t1) + (t3 - t4)) / 2.0;

    // The true current time is the local receive time adjusted by the offset
    Some(t4 + offset)
}

// Write the given Unix time (in seconds) to the system clock via settimeofday
fn set_system_time(unix: f64) -> std::io::Result<()> {
    let secs = unix.floor();
    let mut micros = ((unix - secs) * 1_000_000.0).round() as i64;
    let mut secs = secs as i64;

    // Carry a rounded-up fraction into the seconds field
    if micros >= 1_000_000 {
        secs += 1;
        micros -= 1_000_000;
    }

    let tv = libc::timeval {
        tv_sec: secs as libc::time_t,
        tv_usec: micros as libc::suseconds_t,
    };

    // SAFETY: tv points to a valid timeval for the duration of the call; the time zone argument is null
    let ret = unsafe { libc::settimeofday(&tv, std::ptr::null()) };
    if ret == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

fn main() {
    for server in load_servers() {
        if let Some(unix) = query(&server) {
            // Split into whole seconds and nanoseconds for time-zone conversion and formatting
            let secs = unix.floor();
            let nanos = ((unix - secs) * 1_000_000_000.0).round() as u32;
            let Some(utc) = Utc.timestamp_opt(secs as i64, nanos).single() else {
                continue;
            };
            let local = utc.with_timezone(&Local);
            let formatted = local.format("%Y-%m-%d %H:%M:%S%.3f %z").to_string();

            // Write the corrected time to the system clock before reporting success
            match set_system_time(unix) {
                Ok(()) => {
                    println!("{GREEN}{formatted}{RESET}");
                    println!("{BLUE}Source SNTP server: {server}{RESET}");
                    return;
                }
                Err(e) => {
                    // A valid time was obtained but the clock could not be set; this will not differ across servers, so stop here
                    if e.raw_os_error() == Some(libc::EPERM) {
                        eprintln!("{RED}Setting the system clock requires root privileges; re-run with sudo{RESET}");
                    } else {
                        eprintln!("{RED}Failed to set the system clock: {e}{RESET}");
                    }
                    std::process::exit(1);
                }
            }
        }
    }

    // A full round of polling still yielded no result
    eprintln!("{RED}SNTP time synchronization failed{RESET}");
    std::process::exit(1);
}
