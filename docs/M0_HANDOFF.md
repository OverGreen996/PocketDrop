> 歷史版本文件。目前 1.0 架構請看 [ARCHITECTURE_v1.0.md](ARCHITECTURE_v1.0.md)。

# M0 hand-off — 2026-09-23

> 本文件保存第一輪 M0 結果。使用者後續明確要求 Android APK，新增 Phone Trial v0.2；目前交付請看根目錄 README 與 PHONE_TRIAL_ACCEPTANCE.md。下述「本輪」僅指原 M0，不代表後续手機工作未做。

## 產品優先順序

使用者已補充：**重點是手機與電腦。** 主要成功情境是 Android 分享文字／檔案到 Windows，以及 Windows 放入內容後 Android 立即看到、按需下載。PC ↔ PC 只用於前期協定驗證。保留 LAN-only、無帳號、QR 首次配對、之後自動發現等約束。

本輪仍遵守 M0 範圍；沒有进入 M1，沒有開發 Android 或 iOS。

## 交付

- Tauri 2 / TypeScript / Rust Windows x64 原型與可執行檔。
- 透明無框視窗、原生 Acrylic 開／關、原生圓角、兩張玻璃卡片、手動文字對比、置頂。
- 原生單／多檔拖放 callback、metadata 工作執行緒、拒絕資料夾與 UNC、檔名大小顯示。
- mDNS 啟停、LAN IPv4 介面選擇、未驗證裝置列表，以及獨立 `mdns_probe` 測試程式原始碼。
- 三種桌布 PNG、外部背景測試板、六份主要文件與鎖定檔。

## 已執行的驗證

| 項目 | 本輪狀態 |
|---|---|
| TypeScript 型別檢查／Vite production build | PASS |
| Tauri MSVC release build | PASS，實際產出 Windows x64 EXE |
| Rust tests | PASS，3 項：LAN scope、UNC 拒絕、basename/size/資料夾拒絕 |
| 原生 App 啟動 | PASS，Windows 11 build 26200.9457 + WebView2 |
| Acrylic 呼叫 | API 返回成功；不等於視覺驗收 |
| 原生置頂按鈕 | 操作返回「已永遠置頂」 |
| 完整 UI 縮放 | 已改為可捲動，實機確認底部材質與探索控制可到達；高縮放時無法一屏顯示 |
| 原生 Explorer 單／多檔拖入 | callback 與資料處理已實作；自動化工具拒絕跨應用拖曳，實際 Explorer drop 尚待人工驗收。單元測試不能代替拖放 |
| mDNS 同機 probe | PASS（排除沙箱限制後），兩個獨立程序皆收到 peer_resolved；不是跨 PC 證據 |
| 真實跨 PC mDNS | BLOCKED，沒有第二台受控 PC |
| 深／亮／多色外部原生視窗 | 已截圖，背景切換會改變玻璃色感；Acrylic 模糊細線，關閉後細線清楚 |
| 深／亮／多色實際桌布與移動對照 | 待實機驗收；上述測試板不是桌布 |
| Windows 10／多 DPI／睡醒／長時間效能 | 未驗證 |

## 明確結論

**已證實原生材質能模糊 Widget 外的內容並隨外部背景變色，但目前不得宣稱通過完整 Liquid Glass／M0 驗收。** 截圖還顯示一條額外的 Windows caption 樣貌，與純無框要求不符，必須釐清系統／自動化擷取造成的影響並修正。真正桌布、不同位置、Win10、跨裝置與效能驗收仍待完成。

實際證據：`evidence/01-native-start.png`、`02-acrylic-blue.png`、`03-external-multicolor.png`、`04-external-dark.png`、`05-external-light.png`、`06-blur-off.png`。原始 Windows 截圖，沒有後製成通過效果。

## 風險與未完成

1. Acrylic 使用上游 Windows API 包裝，部分 build 拖移效能差。Windows 關閉透明、節能、RDP/VM 可能使效果變實心；API 不一定回報此狀況。
2. 目前環境採垂直捲動讓底部控制項可達；額外 caption 樣貌尚未解決。桌布自動適應、桌面層級掛載、收合、tray、auto-hide 不在 M0。
3. mDNS 可能受防火牆、AP isolation、VPN/虛擬網卡影響；M0 的私有 IPv4 範圍檢查不等同 Windows 私人網路設定。突然離線依 TTL，沒有可用性保證。
4. M0 身分 UUID 是每次探索新產生，沒有持久身分、TLS、Room、配對或授權。由於没有資料端點，不會把共享文字／檔案暴露給 LAN。
5. 尚未量測全程序樹（包含 WebView2）與 DWM 的 CPU/GPU；不可宣稱近零閒置負載。
6. EXE 未簽章、非安裝包，依賴 WebView2；Windows 可能顯示未知發行者提示。

## 下一步與停止點

由使用者依 `M0_WINDOWS_ACCEPTANCE.md` 驗收 Windows 實機。未得到明確確認前只修 M0，不進 M1。

M0 通過後再確認手機 ↔ 電腦的最短交付路徑：Windows 核心驗證使用跨平台協定資料結構；Android 優先落實 Share Target、Shared Text、檔案 metadata 與按需下載。此處是交接方向，沒有變更原有里程碑授權。
