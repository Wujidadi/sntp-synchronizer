# CLAUDE.md

本檔提供 Claude Code 於本專案工作時的指引

## 專案概觀

單一 binary 的 Rust 命令列程式，依優先順序輪詢多台 SNTP 伺服器取得校時結果
所有邏輯集中於 `src/main.rs`，相依僅 `chrono`

## 核心行為（修改時務必維持）

- 伺服器名單依序查找設定檔：環境變數 `SNTP_SERVERS_FILE` → 工作目錄 `./servers.conf` → `~/.config/sntp-synchronizer/servers.conf`（遵循 `XDG_CONFIG_HOME`），取第一個存在且非空者，全部落空時回退為內建 `SERVERS` 常數
- 依名單順序由上往下查詢，取到第一個有效結果即結束
- 查詢失敗（逾時、解析錯誤、伺服器錯誤、秒數為 0）一律**靜默略過**，不得輸出任何錯誤訊息
- 成功：亮綠色時間 ＋ 藍色來源伺服器
- 全部失敗：紅色 `SNTP time synchronization failed`，並以 exit code 1 結束

## 慣例

- 可調整參數集中於 `src/main.rs` 頂部常數區
- 設定檔採純文字格式（每行一台、`#` 註解、空行略過），以 std 讀取，不引入解析相依
- 設定檔 `servers.conf` 為本地檔案（已列入 `.gitignore`），版控僅保留範例檔 `servers.example.conf`
- `install` 為快速安裝腳本：建立設定檔、連結至 `~/.config/sntp-synchronizer/servers.conf`，再 `cargo install --path .`
- 自行以 `UdpSocket` 處理 NTP 封包，不引入額外網路相依
- 修改後以 `cargo build` 確認可編譯

## 常用指令

```sh
cargo run               # 執行
cargo build --release   # 產出最佳化執行檔
cargo install --path .  # 僅安裝全域可執行檔至 ~/.cargo/bin（不建立設定檔連結）
./install               # 快速安裝：建立設定檔連結並安裝全域可執行檔
```
