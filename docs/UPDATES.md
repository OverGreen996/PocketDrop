# Windows OTA 更新維護

採用 [Tauri 官方 updater](https://v2.tauri.app/plugin/updater/)，Windows NSIS passive 模式，下載後以固定公鑰驗證簽章，再啟動安裝並退出舊程式。installer 由 plugin 加入原安裝目錄及重新啟動參數。沒有自製加密演算法、不允許不安全 TLS、不允許降版；同時要求 manifest 版本與簽章內受保護的版本一致（requireSignedVersion）。

## 本機金鑰

- 私鑰 `.release-private/updater.key`，公鑰 `.release-private/updater.key.pub`。整個目錄已被 Git 排除。
- 公鑰已寫入 `src-tauri/tauri.conf.json`，可公開。
- 務必安全備份私鑰；遺失後無法用目前已發出的程式信任的新簽章更新。不要每個版本重新產生金鑰，也不要把私鑰或其內容放進 GitHub、發佈附件或 Log。
- `scripts/desktop.ps1 build` 優先使用環境變數 `TAURI_SIGNING_PRIVATE_KEY`，否則讀取上述本機路徑；使用 `--ci` 避免無密碼金鑰的互動提示。加密私鑰可另提供 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。
- 此更新簽章與 Windows Authenticode 不同；目前仍無 Authenticode 簽章。

## 每個後續版本

1. 同步更新 npm/Cargo/Tauri 版本，新增 `docs/RELEASE_<version>.md`。
2. 執行 `npm run package`，必須同時成功產生安裝包與 `.exe.sig`。
3. 執行 `powershell -ExecutionPolicy Bypass -File scripts/package-release.ps1 -SkipBuild -WindowsOnly`。
4. 發布 GitHub Release 時先建立 draft，上傳 `PocketDrop-<version>-Windows-x64-Setup.exe`、同名 `.exe.sig`、`latest.json`、`SHA256SUMS.txt`，驗證附件後才設成正式 latest。
5. 更新來源為 `https://github.com/OverGreen996/PocketDrop/releases/latest/download/latest.json`。後續 latest release（即使只更新 Android）也必須攜帶有效 Windows 更新清單，或不要將其設成 latest。
6. 禁止覆蓋已發布安裝包而沿用舊簽章。每次重建都必須重新簽章並重新生成 manifest。

## 驗證

- `npm run build`、`npm run test:rust`。
- `node scripts/test-updates-ui.mjs <playwright/index.mjs 的路徑>`：更新提示、稍後、安裝順序、已最新、離線重試、簽章錯誤、草稿無法保存均採注入測試。
- 設 `PD_UPDATE_PACKAGE` 為安裝包完整路徑（旁邊必須有 `.sig`），執行 `cargo test --manifest-path src-tauri/Cargo.toml --locked signed_download_rejects -- --ignored`。測試真實 updater 的版本比較、下載與簽章驗證、遭竄改拒絕、無效版本及離線，不執行安裝器。
- 發布後可執行 `cargo test --manifest-path src-tauri/Cargo.toml --locked published_update_download_verifies -- --ignored`，從正式 GitHub 下載並驗證最新包，不執行安裝。
- 僅測試用 loopback fixture 使用 HTTP；production 是 HTTPS 並且保持憑證驗證。

## 行為界線

每次啟動只檢查一次，非背景推送服務。查詢逾時 12 秒，下載逾時 180 秒。使用者同意後才下載，下載中顯示進度，不提供斷點續傳。官方 updater 會將更新包讀入記憶體驗證；目前安裝包很小，這與 Room 大檔案串流不同。

安裝前保存未分享草稿並攔截本機進行中的下載；來源端正在供應其他装置的下載仍可能中斷，因此提示先完成傳輸。資料與應用識別沿用。舊版本没有更新器，必須手動安裝啟用版一次。Android OTA 不在本次實作。
