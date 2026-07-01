use std::net::{ToSocketAddrs, UdpSocket};
use std::time::Duration;

use chrono::{Local, TimeZone, Utc};

// 依優先順序排列的 SNTP 伺服器名單，由上往下依序嘗試
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

// NTP 埠號
const NTP_PORT: u16 = 123;

// 1900-01-01 到 1970-01-01 之間的秒數差，用於把 NTP 時間換算成 Unix 時間
const NTP_UNIX_DELTA: u64 = 2_208_988_800;

// 單次查詢的逾時秒數
const TIMEOUT_SECS: u64 = 3;

// ANSI 色碼
const GREEN: &str = "\x1b[92m"; // 亮綠色
const BLUE: &str = "\x1b[94m"; // 藍色
const RED: &str = "\x1b[91m"; // 紅色
const RESET: &str = "\x1b[0m";

// 向單一 SNTP 伺服器查詢，成功時回傳（Unix 秒數，奈秒）
fn query(server: &str) -> Option<(i64, u32)> {
    // 解析主機名稱，取第一個可用位址
    let addr = (server, NTP_PORT).to_socket_addrs().ok()?.next()?;

    // 綁定本機任意埠，並設定收送逾時
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    let timeout = Some(Duration::from_secs(TIMEOUT_SECS));
    socket.set_read_timeout(timeout).ok()?;
    socket.set_write_timeout(timeout).ok()?;
    socket.connect(addr).ok()?;

    // 組出 48 位元組的 NTP 請求封包，首位元組 0x1B 代表 LI＝0、VN＝3、Mode＝3（client）
    let mut request = [0u8; 48];
    request[0] = 0x1B;
    socket.send(&request).ok()?;

    // 接收回應並確認長度
    let mut response = [0u8; 48];
    let received = socket.recv(&mut response).ok()?;
    if received < 48 {
        return None;
    }

    // Transmit Timestamp 位於偏移量 40，前 4 位元組為秒數、後 4 位元組為小數部分
    let seconds = u32::from_be_bytes([response[40], response[41], response[42], response[43]]) as u64;
    let fraction = u32::from_be_bytes([response[44], response[45], response[46], response[47]]) as u64;

    // 秒數為 0 視為無效回應（Kiss-o'-Death 或錯誤封包）
    if seconds == 0 {
        return None;
    }

    let unix_secs = seconds.checked_sub(NTP_UNIX_DELTA)? as i64;
    let nanos = (fraction * 1_000_000_000 >> 32) as u32;

    Some((unix_secs, nanos))
}

fn main() {
    for server in SERVERS {
        if let Some((secs, nanos)) = query(server) {
            // 換算成本地時區的時間並格式化輸出
            if let Some(utc) = Utc.timestamp_opt(secs, nanos).single() {
                let local = utc.with_timezone(&Local);
                let formatted = local.format("%Y-%m-%d %H:%M:%S%.3f %z").to_string();

                println!("{GREEN}{formatted}{RESET}");
                println!("{BLUE}來源 SNTP 伺服器：{server}{RESET}");
                return;
            }
        }
    }

    // 全部輪詢一次仍取不到結果
    eprintln!("{RED}SNTP time synchronization failed{RESET}");
    std::process::exit(1);
}
