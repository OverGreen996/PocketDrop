# PocketDrop · Android ↔ Windows Phone Trial 0.2.2

0.2.2 修正 Windows 玻璃底層矩形殘邊，並改善 Android 關閉重開後的自動重連。**兩端都要更新；APK 直接覆蓋安裝，不解除安裝、不清資料，保留原配對。** Windows 改用系統較小的原生圓角，完整效果目前需要 Windows 11 22H2 以上。詳細結果與限制見 [0.2.2 修正記錄](docs/FIX_0.2.2.md)。

把文字貼進去、檔案丟進去，自己的手機與電腦就能取用。

依使用者後續要求，工作範圍已從 M0 擴充成可安裝 APK 的手機連線測試版。**這版由 Windows 保存 Room，電腦必須保持開啟；不是最終去中心化架構。** 原 M0 交付包保留，舊說明位於 [M0 文件封存](docs/M0_README_ARCHIVE.md)。

## 開始使用

1. 關閉舊版，完整解壓 `artifacts/PocketDrop-Phone-Trial-0.2.2-windows-x64.zip`，執行 `PocketDrop.exe`。需要 Windows x64 與 Edge WebView2 Runtime。不要直接從 ZIP 暫存目錄啟動。
2. 將 `artifacts/PocketDrop-Android-0.2.2.apk` 複製到 Android 手機，點開覆蓋安裝。最低 Android 8，建議 Android 10 以上。
3. 兩端連到同一個 Wi-Fi／LAN。若 Windows 顯示防火牆提示，允許信任的私人網路，讓手機可連入。
4. 已配對者直接開啟手機 App 等待重新連線。只有第一次使用才需要電腦按「配對手機」；手機開啟 PocketDrop →「裝置」→「掃描電腦 QR Code」。邀請有效 5 分鐘，只能使用一次。
5. 任一端貼上文字並按「分享文字」，另一端立即取得更新並可複製。
6. 將檔案拖進電腦 Widget，手機「檔案」就會列出；按「下載」才傳送內容。
7. 手機按「加入手機檔案」，或從相簿／檔案管理器「分享 → PocketDrop」，會上傳至電腦的「下載／PocketDrop」。每筆接收檔案各有一個子資料夾，避免重名覆蓋。

Android 10 以上下載到 `Downloads/PocketDrop`。Android 8–9 存入 App 專用下載資料夾，可透過下載完成的「開啟」取用；解除安裝會移除 App 專用檔案。這是開發測試簽章 APK，不是 Play 商店正式版。

## 已實作

- Windows 原生 Acrylic 透明無框 Widget；拖入單檔／多檔、文字區、檔案區、配對裝置列表與撤銷。
- QR 短效一次性邀請；Windows DPAPI 保護 TLS 私鑰；Android Keystore 保護配對憑證；憑證 SHA-256 釘選。
- HTTPS + WebSocket；已配對手機才可讀寫資料。mDNS 只公告裝置識別與能力，不公告邀請或內容。
- SQLite 保存文字、最近 20 筆修改、檔案資訊、信任與刪除標記。PC 決定遞增文字順序，不依賴裝置時鐘排序。
- 串流上傳／下载、速度與剩餘時間、取消、Range API、背景 SHA-256。檔名防穿越，網路回應不包含來源本機路徑。
- Android 系統 Share Target、QR 掃碼、重新開啟後保存配對，mDNS 找回電腦位址。

## 驗證與限制

自動測試結果與逐項手機驗收請看 [PHONE_TRIAL_ACCEPTANCE.md](docs/PHONE_TRIAL_ACCEPTANCE.md)。沒有連接實體 Android 裝置，尚未宣稱手機實測通過。

- 此版不支援手機離線寫入後合併、不提供手機常駐背景服務或手機端檔案伺服器。
- Android 上傳會把檔案存到 PC；和最終「檔案保留在各來源裝置、隨選直取」架構不同。
- 續傳 API 已預留，Android 暫不自動續傳。歷史有保存，但還沒有歷史選取介面或保留數設定。
- 每個檔案最多 16 GiB，最多 200 筆有效檔案，最多兩路上傳。未進行 10 GiB／長時間待機測試。
- PC 在執行中切換網路／IP 改變時，重新開啟 PC 版；或在「連線與外觀設定」選網路後按「套用」。手機會重新探索。
- 路由器的訪客網路、裝置隔離、VPN、防火牆可能阻擋連線；本版不透過網際網路繞過它們。
- Win10、實際桌布的明暗移動測試與跨 DPI 多螢幕仍待驗收；不把 M0 的測試板結果當成全部玻璃驗收已通過。

安全與協定細節：[PHONE_TRIAL_PROTOCOL.md](docs/PHONE_TRIAL_PROTOCOL.md)。

## 開發與重新建置

```powershell
npm.cmd ci
npm.cmd run package
npm.cmd run test:rust
powershell -ExecutionPolicy Bypass -File scripts/android-build.ps1
node scripts/phone-smoke.mjs
powershell -ExecutionPolicy Bypass -File scripts/package-phone.ps1
```

Android 使用 JDK 17、Gradle 8.11.1、Android SDK 35／Build Tools 35.0.0、AGP 8.9.2。本工作區的工具位於 `.tools`；乾淨環境可用 Android Studio 開啟 `android`，配置相同版本，執行 `assembleDebug lintDebug testDebugUnitTest`。Rust 使用 MSVC 工具鏈，需 Visual Studio C++ Build Tools 與 Windows SDK。建置前請關閉正在執行的測試 EXE。

本工作區未初始化 Git；所有舊 M0 文件與交付壓縮檔保留。未進入正式 M1–M8 全產品開發。
