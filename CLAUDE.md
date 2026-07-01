# CLAUDE.md

本檔提供 Claude Code 於本專案工作時的指引

## 專案概觀

單一 binary 的 Rust 命令列程式，依優先順序輪詢多台 SNTP 伺服器取得校時結果
所有邏輯集中於 `src/main.rs`，相依僅 `chrono`

## 核心行為（修改時務必維持）

- 依 `SERVERS` 名單順序由上往下查詢，取到第一個有效結果即結束
- 查詢失敗（逾時、解析錯誤、伺服器錯誤、秒數為 0）一律**靜默略過**，不得輸出任何錯誤訊息
- 成功：亮綠色時間 ＋ 藍色來源伺服器
- 全部失敗：紅色 `SNTP time synchronization failed`，並以 exit code 1 結束

## 慣例

- 可調整參數集中於 `src/main.rs` 頂部常數區
- 自行以 `UdpSocket` 處理 NTP 封包，不引入額外網路相依
- 修改後以 `cargo build` 確認可編譯

## 常用指令

```sh
cargo run              # 執行
cargo build --release  # 產出最佳化執行檔
cargo install --path . # 安裝為全域可執行檔至 ~/.cargo/bin
```
