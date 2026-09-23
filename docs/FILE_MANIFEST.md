# 檔案清單

| 路徑 | 用途 |
|---|---|
| `README.md` | 安裝、啟動、建置、範圍說明 |
| `package.json` / `package-lock.json` | npm 指令與鎖定依賴 |
| `tsconfig.json` / `vite.config.ts` / `index.html` | 前端編譯與本機 dev server |
| `src/main.ts` | PocketDrop Widget、QR、實際文字與檔案清單、裝置撤銷 |
| `src/m0-reference.ts` | 舊 M0 畫面原始碼封存，不納入目前入口 |
| `src/style.css` | 透明表面、細亮邊、卡片、拖放回饋；不模擬桌布 |
| `public/backdrop.html` / `backdrop.js` | 獨立外部視窗測試背景 |
| `src-tauri/Cargo.toml` / `Cargo.lock` / `build.rs` | Rust 建置與鎖定依賴 |
| `src-tauri/tauri.conf.json` | 原生透明無框視窗、CSP |
| `src-tauri/capabilities/default.json` | 最小視窗權限 |
| `src-tauri/src/main.rs` | 原生窗、Acrylic、圓角、拖放與生命周期 |
| `src-tauri/src/native_frame.rs` | UI 執行緒原生邊框、焦點重繪抑制、實際尺寸圓角 |
| `src-tauri/src/files.rs` | 只讀 metadata、限量、路徑隱私、測試 |
| `src-tauri/src/discovery.rs` | mDNS allowlist、限定介面、peer 事件、清理 |
| `src-tauri/src/phone.rs` | Room TLS／SQLite／邀請／授權／WebSocket／串流與 Range API |
| `android/settings.gradle` / `build.gradle` / `gradle.properties` | Android 專案與 AGP 建置設定 |
| `android/app/build.gradle` | SDK 版本、套件、測試依賴 |
| `android/app/src/main/AndroidManifest.xml` | 權限、Share Target、FileProvider、禁用明文與備份 |
| `android/app/src/main/java/local/pocketdrop/android/MainActivity.java` | 手機文字／檔案／裝置 UI、掃碼、分享、MediaStore 傳輸 |
| `android/app/src/main/java/local/pocketdrop/android/RoomClient.java` | OkHttp HTTPS、憑證 pin、Room 授權 |
| `android/app/src/main/java/local/pocketdrop/android/Vault.java` | Keystore 身分與加密 credential 保存 |
| `android/app/src/main/java/local/pocketdrop/android/Nearby.java` | Android NSD／multicast 與已配對 PC 探索 |
| `android/app/src/main/java/local/pocketdrop/android/LanRules.java` | 私有端點與檔名驗證 |
| `android/app/src/main/res/` | Android 主題、圖示、FileProvider 範圍、備份排除規則 |
| `android/app/src/test/` | LAN 與惡意檔名測試 |
| `src-tauri/examples/mdns_probe.rs` | 20 秒獨立探索測試工具，不是檔案服務 |
| `src-tauri/icons/` | 原型圖示與建置資源 |
| `scripts/desktop.ps1` | 自動找專案 Rust／PATH，dev/build/test/check |
| `scripts/make-fixtures.ps1` | 生成測試桌布、無敏感測試檔案與圖示 |
| `scripts/package-artifact.ps1` | 封裝 EXE、文件、素材、證據與 SHA256 |
| `scripts/android-build.ps1` | 專案內 Android 工具鏈、APK、Lint、單元測試 |
| `scripts/phone-smoke.mjs` | Windows loopback HTTPS 的獨立授權／資料測試 |
| `scripts/verify-qr-screenshot.mjs` | 解碼實際 Windows QR 截圖，不輸出邀請 token |
| `scripts/package-phone.ps1` | Phone Trial APK + Windows ZIP + SHA256 |
| `fixtures/` | 深／亮／多色 PNG、txt/stl、資料夾拒絕樣本 |
| `docs/ARCHITECTURE_v0.1.md` | M0 架構、安全邊界、手機↔電腦優先 |
| `docs/M0_TEST_PROFILE.md` | 環境、素材、測試步驟 |
| `docs/M0_WINDOWS_ACCEPTANCE.md` | 驗收表、版本差異、簽核 gate |
| `docs/M0_HANDOFF.md` | 真實結果、限制、停止點 |
| `docs/FILE_MANIFEST.md` | 本清單 |
| `docs/PHONE_TRIAL_PLAN.md` | 使用者變更範圍與 PC-hosted 過渡方案 |
| `docs/PHONE_TRIAL_PROTOCOL.md` | 實際 API、安全模型、偏離正式架構之處 |
| `docs/PHONE_TRIAL_ACCEPTANCE.md` | 自動驗證結果與待完成手機實測 |
| `docs/FIX_0.2.2.md` | 原生整層圓角、Android 重連修正與升級驗收 |
| `android/app/src/test/java/local/pocketdrop/android/ReconnectTest.java` | 正式 RoomClient 的 TLS、冷啟動、端點變更、取消與 pin 回歸 |
| `evidence/windows-0.2.2/native-rounded-on-external-board.jpg` | 含視窗外側背景的整層圓角截圖 |
| `docs/M0_README_ARCHIVE.md` | M0 舊 README 保存，路徑以專案根目錄為準 |
| `evidence/` | 實際截圖與結果紀錄；不包含仿造通過的效果圖 |
| `artifacts/` | 執行檔、ZIP、SHA256（生成物，不納入原始碼） |
| `.gitignore` | 排除依賴、快取、建置輸出 |

`.tools/`、`.npm-cache/`、`node_modules/`、`dist/`、`src-tauri/target/` 均為本機可重建產物，不應當成產品原始碼提交。Tauri 自動產生的 `src-tauri/gen/schemas` 是權限 schema，不是 Room 資料庫。

初始 Git 狀態：資料夾不是 Git repository；本輪未替使用者建立遠端、提交或推送。

## 1.0 新增與發佈

| 路徑 | 用途 |
|---|---|
| `src-tauri/src/phone/pairing.rs` | SRP-6a 驗證碼配對、短效／限次／TLS 身分綁定、驗證測試 |
| `src-tauri/src/phone/peer.rs` | Windows 加入 Room、pin、DPAPI、mDNS、metadata、按需回讀及下載 |
| `android/app/src/main/java/local/pocketdrop/android/CodePairing.java` | Bouncy Castle SRP6Client，驗證碼配對 |
| `android/app/src/test/java/local/pocketdrop/android/CodePairingInteropTest.java` | Android Java ↔ Windows EXE 真實 TLS 配對及重啟整合檢查 |
| `docs/ARCHITECTURE_v1.0.md` | 目前架構、權限、傳輸與平台邊界 |
| `docs/RELEASE_1.0.0.md` | 發佈範圍、檢查及明確限制 |
| `scripts/package-release.ps1` | 建置並整理正式 Windows 安裝包、Android APK 及 SHA256SUMS |
| `artifacts/release-1.0.0/` | 本機安裝包、APK、校驗值；以 GitHub Release 附件發佈，不放進 Git 原始碼 |

排除：工具鏈、建置目錄、私鑰／keystore、DPAPI、SQLite、邀請資料、使用者截圖與本機測試資料。
