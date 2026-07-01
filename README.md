# sntp-synchronizer

依優先順序輪詢多台 SNTP 伺服器，取得最新校時結果的 Rust 命令列程式

## 功能

- 依名單順序由上往下逐台查詢，一取到有效結果即結束
- 成功時以**亮綠色**顯示時間、**藍色**顯示來源伺服器
- 逾時、解析失敗或伺服器錯誤一律靜默略過，直接嘗試下一台
- 全部輪詢一次仍失敗，才以**紅色**顯示 `SNTP time synchronization failed`，並以 exit code 1 結束

## 建置與執行

```sh
cargo run              # 開發模式執行
cargo build --release  # 產出最佳化執行檔於 target/release/
cargo install --path . # 安裝為全域可執行檔至 ~/.cargo/bin
```

安裝後可直接以指令名稱執行：

```sh
sntp-synchronizer
```

## 輸出範例

```
2026-07-01 18:53:38.534 +0800
來源 SNTP 伺服器：time.stdtime.gov.tw
```

## 實作說明

- 以 std 的 `UdpSocket` 自行組出 48 位元組 NTP 請求封包（首位元組 `0x1B`＝LI 0／VN 3／Mode 3），讀取偏移量 40 的 Transmit Timestamp
- 時間換算與時區格式化使用 `chrono`，色彩以 ANSI 色碼直接處理

## 可調整項

集中於 `src/main.rs` 頂部常數：

| 常數           | 說明                       |
| -------------- | -------------------------- |
| `SERVERS`      | 伺服器名單與優先順序       |
| `TIMEOUT_SECS` | 單台查詢逾時秒數（預設 3） |
| `GREEN` 等色碼 | 各狀態顯示色彩             |
