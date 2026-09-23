# PocketDrop · M0 Windows prototype（舊版文件封存，路徑以專案根目錄為準）

PocketDrop 是 LAN 裡的共享暫存盒：文字貼進去，檔案丟進去，自己的裝置馬上都能拿。

產品重點是 **Android 手機 ↔ Windows 電腦**；PC ↔ PC 只是前期技術驗證手段。

**這份交付僅為 M0 技術原型，不是可共享資料的 MVP。** 沒有 Room、配對、文字同步、下載 API、資料庫或手機版。原生玻璃與跨 PC mDNS 必須依驗收表確認後，才可討論 M1。

## 執行

Windows x64 + Microsoft Edge WebView2 Runtime。建置後執行 `src-tauri/target/release/pocketdrop-m0.exe`，或使用 `artifacts/PocketDrop-M0-windows-x64.zip` 中的執行檔。不需帳號，不需固定 IP。

1. 拖曳 PocketDrop 標誌／名稱移動 Widget。文字區可選取；檔案可拖入整個 Widget。
2. 從檔案總管拖入 `fixtures/sample.txt`；再一起拖入 `sample.txt` 與 `sample.stl`。
3. 「背景材質」可切換原生 Acrylic 與關閉模糊；◐ 切換深／亮文字。◇ 切換置頂。
4. 開啟外部背景測試板，先將 Widget 置頂，再移到不同色區觀察。這是獨立原生視窗，不是 Widget 內的 CSS 背景，也不是桌布實测的替代品。
5. 正式桌布測試：將 `fixtures/wallpaper-dark.png`、`wallpaper-light.png`、`wallpaper-multicolor.png` 分別設為 Windows 桌布，依 [驗收表](docs/M0_WINDOWS_ACCEPTANCE.md) 截圖記錄。測試後還原原桌布。
6. 同 LAN 的兩台 PC 各啟動一份程式。在測試面板選實際 Wi-Fi／乙太網路介面，按「開始」。防火牆若詢問，僅允許信任的私人網路；用途是找附近的測試裝置。不要關閉防火牆，也不要允許公用網路。

探索結果一律標示「未驗證」，不是 Room 成員。M0 網路沒有任何文字／檔案讀取端點。測試面板顯示介面/IP 是工程驗證用途，不是最終產品流程。

## 從原始碼建置

先安裝 Node.js 22.12+、Rust stable **MSVC**、Microsoft C++ Build Tools（Desktop development with C++、Windows SDK）、WebView2。參考 [Tauri 官方 prerequisites](https://v2.tauri.app/start/prerequisites/)。

```powershell
npm.cmd ci --cache .npm-cache
npm.cmd run package
npm.cmd run test:rust
npm.cmd run desktop
```

`package` 建置 release 執行檔、不產生安裝程式。`desktop` 啟動開發版，Vite 僅綁定 127.0.0.1:1420。不要把 Vite 開給 LAN。

建置後執行 `powershell -ExecutionPolicy Bypass -File scripts/package-artifact.ps1` 生成含測試素材／文件／截圖的 ZIP。解壓後執行 `PocketDrop-M0.exe`。ZIP 的 SHA256SUMS.txt 可校驗執行檔。

本工作區的 Rust 安裝在 `.tools/cargo`、`.tools/rustup`；`scripts/desktop.ps1` 會自動採用，其他電腦則使用 PATH 中的 Rust。依賴鎖定於 `package-lock.json` 和 `src-tauri/Cargo.lock`。`.tools`、`node_modules`、建置快取不需交付。

重新產生測試素材：`powershell -ExecutionPolicy Bypass -File scripts/make-fixtures.ps1`。這個指令只生成檔案，不更改 Windows 桌布。

## 文件與狀態

- [架構與安全邊界](docs/ARCHITECTURE_v0.1.md)
- [測試環境與重現方式](docs/M0_TEST_PROFILE.md)
- [Windows 驗收與版本差異](docs/M0_WINDOWS_ACCEPTANCE.md)
- [本輪結果、限制與交接](docs/M0_HANDOFF.md)
- [檔案清單](docs/FILE_MANIFEST.md)

M0 的「Acrylic API 已接受」僅代表函式呼叫返回成功，**不代表作業系統真的顯示模糊**。原生效果失敗會明示，不會以固定灰底或背景截圖冒充通過。尚未達成完整實機驗收前，不得將本原型當成已符合 Liquid Glass 的產品。
