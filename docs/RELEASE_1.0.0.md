# PocketDrop 1.0.0 發佈紀錄

## 交付範圍

- Windows NSIS x64 安裝包，產品名稱 PocketDrop，無測試版介面標籤。
- Android Release APK：versionCode 4、versionName 1.0.0、debuggable=false。
- Windows 建立／加入 Room；Windows 與 Android 均提供驗證碼及 QR 配對。Windows 可用攝影機或 QR 圖片。
- 保存原有應用程式 ID、資料及 Android 簽章，支持覆蓋更新。
- 桌面檔案 metadata 宣告、來源狀態、按需串流轉送、進度及取消；手機分享入口沿用上傳到 Room 電腦的流程。

## 已執行檢查

- TypeScript 與 Vite 生產建置。
- Rust 12 項測試：包含真實私有網路介面上兩個獨立裝置身分的 HTTPS、mDNS、驗證碼／QR 配對、文字、隨選下載、Range、離線／恢復、連接埠改變、撤銷及 tombstone。
- 原有 HTTPS API 13 項情境，包括未配對阻擋、一次性邀請、文字／檔案、大小與路徑驗證、Range、取消及撤銷。
- Android 7 項 JVM 測試，含正式 Java SRP/TLS 用戶端對實際 Windows EXE 的跨語言配對：錯誤碼阻擋、正確配對、分享文字、邀請重用阻擋、重啟 Windows 服務及重建 Android 用戶端後原憑證與權杖繼續有效。該整合測試需設定 PD_INTEROP_EXE，沒有設定時明確跳過。
- Android Release Lint、APK 簽章驗證及 manifest 檢查。
- Windows 原生畫面檢查：無傳統標題列、原生背景色穿透、邀請驗證碼／QR 視窗、另一台電腦加入入口及圖片選擇入口。未操作使用者既有文字／檔案；既有配對仍可見。

## 邊界與已知限制

這些檢查在同一台 Windows 開發機的獨立測試資料中執行；未將 JVM 測試冒稱 Android 實機或兩台實體 PC 的長時間驗收。未完成 Windows 安裝精靈的實際安裝／移除循環、手機攝影機掃碼或各廠牌路由器相容性矩陣。相機入口受 Windows／裝置相機權限影響，亦提供 QR 圖片及驗證碼替代入口。

本次依使用者決定採 PC-hosted Room；主機必須常開。Android 尚無背景常駐服務。沒有跨網際網路功能、iOS、多主機共識、Tray／自動隱藏、下載續傳介面或歷史 UI。

Windows 安裝包未有 Authenticode 簽章。Android 為非 debuggable Release 建置，為保持既有安裝可更新，仍沿用早期本機產生的簽章憑證（憑證名稱 Android Debug）；不是 Google Play 發佈版本。私鑰與 keystore 不上傳 GitHub。SHA-256 公開簽章指紋：`1b44b5384101836bff52967d99f6711ea77bc532d5f6ec01bc82d7d2e5680438`。

使用成熟函式庫不等同完成獨立安全稽核；本次沒有第三方安全稽核。原始 M0 文件保留為歷史記錄，不代表 1.0 的現況。
