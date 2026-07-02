# sntp-synchronizer

依優先順序輪詢多台 SNTP 伺服器，取得最新校時結果的 Rust 命令列程式

## 功能

- 依名單順序由上往下逐台查詢，一取到有效結果即結束
- 成功時以**亮綠色**顯示時間、**藍色**顯示來源伺服器
- 逾時、解析失敗或伺服器錯誤一律靜默略過，直接嘗試下一台
- 全部輪詢一次仍失敗，才以**紅色**顯示 `SNTP time synchronization failed`，並以 exit code 1 結束

## 快速安裝

一鍵完成「建立設定檔 → 連結至使用者設定目錄 → 安裝全域可執行檔」：

```sh
./install
```

腳本依序完成：

1. 本地無 `servers.conf` 時，由 `servers.example.conf` 複製建立
2. 建立符號連結 `~/.config/sntp-synchronizer/servers.conf` 指向本地 `servers.conf`
3. 執行 `cargo install --path .` 安裝至 `~/.cargo/bin`

完成後於任意目錄皆可執行 `sntp-synchronizer`，並經上述連結讀到同一份設定檔

## 建置與執行

```sh
cargo run               # 開發模式執行
cargo build --release   # 產出最佳化執行檔於 target/release/
cargo install --path .  # 僅安裝全域可執行檔至 ~/.cargo/bin（不建立設定檔連結）
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

## 伺服器名單設定

伺服器名單可透過設定檔調整，設定檔屬本地檔案不納入版控；版控中僅提供範例檔 `servers.example.conf`
未使用快速安裝時，可自行從範例檔複製建立設定檔後再依需求調整：

```sh
cp servers.example.conf servers.conf
```

程式依下列順序查找設定檔，取第一個存在且非空者，全部落空則回退內建預設名單：

1. 環境變數 `SNTP_SERVERS_FILE` 指定的路徑
2. 工作目錄下的 `./servers.conf`（方便開發與專案內執行）
3. `~/.config/sntp-synchronizer/servers.conf`（全域安裝的標準位置，可為符號連結；亦遵循 `XDG_CONFIG_HOME`）
4. 內建預設名單（`src/main.rs` 的 `SERVERS` 常數）

設定檔格式：每行一個主機名稱，依優先順序由上往下嘗試；`#` 起始為註解，空行忽略

```sh
SNTP_SERVERS_FILE=/path/to/my-servers.conf sntp-synchronizer
```

## 實作說明

- 以 std 的 `UdpSocket` 自行組出 48 位元組 NTP 請求封包（首位元組 `0x1B`＝LI 0／VN 3／Mode 3），讀取偏移量 40 的 Transmit Timestamp
- 時間換算與時區格式化使用 `chrono`，色彩以 ANSI 色碼直接處理

## 可調整項

集中於 `src/main.rs` 頂部常數：

| 常數           | 說明                           |
| -------------- | ------------------------------ |
| `SERVERS`      | 內建預設名單（設定檔的回退值） |
| `SERVERS_FILE` | 設定檔預設路徑                 |
| `TIMEOUT_SECS` | 單台查詢逾時秒數（預設 3）     |
| `GREEN` 等色碼 | 各狀態顯示色彩                 |
