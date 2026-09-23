> 歷史版本文件。目前 1.0 架構請看 [ARCHITECTURE_v1.0.md](ARCHITECTURE_v1.0.md)。

# M0 測試環境

## 本輪環境

- 日期：2026-09-23，Asia/Taipei。
- Windows registry：DisplayVersion 25H2，CurrentBuild 26200，UBR 9457。ProductName 仍寫 Windows 10 Pro，不據此誤判；build 對應 Windows 11。
- Node.js 24.21.0 / npm 11.19.0。
- Rust 1.98.1 stable x86_64-pc-windows-msvc（工作區 `.tools`）。
- Microsoft Visual Studio 2022 Build Tools，C++ workload 與 Windows SDK。
- 初始工作區是空資料夾，不是 Git repository；沒有覆寫既有檔案。
- 未提供第二台受控 PC 或 Windows 10 測試機；跨實機結果不得推定通過。

## 測試素材

`fixtures/wallpaper-{dark,light,multicolor}.png`：1920 × 1080 PNG，三個色區和 24px 週期細線，可比較模糊開關。這些是**Windows 桌布測試輸入**，不是貼在 Widget 裡的背景。

`public/backdrop.html`：獨立 native window 中的背景測試板。只用來隔離「模糊是否能影響 WebView 外部」；正式驗收還需實際桌布與前景／失焦測試。

`fixtures/sample.txt`、`sample.stl`：無敏感內容。`folder-rejection`：必須拒絕的資料夾。

## 每台 PC 記錄

填寫：OS build、GPU/driver、WebView2 version、縮放比例、螢幕解析度、HDR、透明效果開關、節能模式、RDP/VM 或本機桌面、網卡類型、Windows network profile。不要把真實主機名、公開 IP 或私人內容放进分享的驗收附件。

## 執行順序

1. `npm.cmd ci --cache .npm-cache` → `npm.cmd run package` → `npm.cmd run test:rust`。
2. 執行 release EXE；記錄啟動、圓角、文字選取、拖移、最小化、關閉。
3. 玻璃：多色圖 A/B、暗圖、亮圖、前景／失焦、透明效果關閉、100%／150% DPI、多螢幕。
4. 檔案：單檔、多檔、中文、長檔名、0 byte、大檔、資料夾、取消拖曳、100+ 項目。只檢查 metadata，不應有與檔案大小等比例的 RAM 增長。
5. 發現：兩台真 PC 同一 LAN、同一版本、選實體網卡、兩邊開始。檢查雙向顯示不同 UUID，停止一邊看移除；不要把同機兩 process 算作兩台 PC。
6. 斷網／睡醒／DHCP 改變：記錄 M0 限制，必要時重啟探索；不宣稱已實作 M1 reconnect。
7. 閒置 60 秒後量測至少 60 秒，分別記錄 PocketDrop、其 WebView2 子程序與 DWM CPU/GPU。用工作管理員／效能工具記錄平均和峰值；M0 建議閒置 CPU <1%（含 WebView2，硬體相關），模糊開關不應造成持續明顯 GPU 升高。正式判準待指定硬體。

## 證據規則

原生截圖放 `evidence/`，記錄模式與背景。截圖只可證明當下視覺，無法證明連續移動流暢；另記錄拖動觀察／錄影。外部測試板、瀏覽器預覽與真正桌布證據必須分開。API success、編譯通過、同機發現，都不能替代硬體验收。
