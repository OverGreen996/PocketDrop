# PocketDrop

區域網路裡的共享暫存盒。文字貼進去，檔案丟進去，自己的裝置就能取用。

**Windows 1.0.5 · Android 1.0.2**

## 下載與安裝

到 [Releases](https://github.com/OverGreen996/PocketDrop/releases/latest) 下載：

- **PocketDrop-1.0.5-Windows-x64-Setup.exe**：Windows 安裝包，安裝於目前使用者帳號。
- [**PocketDrop-1.0.2-Android.apk**](https://github.com/OverGreen996/PocketDrop/releases/download/android-v1.0.2/PocketDrop-1.0.2-Android.apk)：複製到 Android 手機後點擊安裝。已有 PocketDrop 時直接覆蓋更新，請勿先解除安裝。
- `SHA256SUMS.txt`：下載檔案校驗值。

Windows 清晰外觀以 **Windows 10／11、x64** 為目標，不依賴原生玻璃。玻璃需要 Windows 11 22H2 以上；不承諾 Windows 7／8 支援。Android 需要 **Android 8.0 以上**。
Windows 安裝包尚未使用 Authenticode 簽章；Microsoft WebView2 尚未安裝時，安裝程序需要網路下載其官方執行環境。PocketDrop 的文字、檔案與配對只走 LAN。

## 開始使用

1. 在一台常開的電腦啟動 PocketDrop。首次啟動會準備自己的 Room。
2. 按 **邀請裝置**，畫面會顯示電腦編號、8 位驗證碼及 QR Code。
3. 第二台電腦按 **加入既有 Room**；Android 到 **裝置 → 輸入驗證碼加入**，選擇相同編號並輸入驗證碼。也可掃描 QR；沒有攝影機的電腦可選取 QR 圖片。
4. 所有裝置使用同一個 Wi-Fi／LAN。配對完成後會保存身分，重新開啟、換 IP 或連接埠改變時自動尋找原有電腦，不用重掃。
5. 貼上文字後按 **分享文字**。將檔案直接拖進 Windows Widget；其他裝置按 **下載** 才取得檔案。

Windows 防火牆提示是讓自己的裝置能在私人網路連線；請允許信任的私人網路。訪客 Wi-Fi 的裝置隔離可能阻止互相發現。

視窗可拖曳四邊及四角調整大小（最小 400 × 480）。文字區依內容與換行自動增高，最多採計前 1,000 個字元，並以視窗高度 65%／600 px 為上限（最小 180 px）。超過時在文字區捲動；內容不截斷，仍可框選、Ctrl+A 及複製。

## 這一版的運作方式

**建立 Room 的電腦必須保持開啟。** 這是目前版本採用的運作方式，尚未提供無中央裝置的 Room。

- Windows 電腦拖入的檔案只登錄資訊，不預先複製。下載時由 Room 電腦轉送來源電腦的串流，不落地保存中繼副本。
- Android 分享／加入檔案會上傳至 Room 電腦保存；接收裝置仍按需下載。
- 電腦下載位置為 `下載/PocketDrop`。Android 10 以上存入 `Downloads/PocketDrop`；Android 8–9 存入 App 專用下載資料夾。
- 來源電腦離線後約 12 秒內顯示不可用；重新上線會恢復。已移除紀錄不會被來源重新宣告而復活。
- 每台來源電腦最多 200 筆檔案；單檔最多 16 GiB，每次拖入最多 100 個。不支援資料夾，請先壓縮成 ZIP。
- 支援進度、取消、串流、SHA-256、HTTP Range；尚未提供下載中斷後的自動續傳介面。
- Android 開啟 App 後連線；不提供背景常駐同步服務。
- 按右上角 **◐ → 外觀**，選擇「清晰深色」「清晰淺色」或「原生玻璃」。預設清晰深色，不透明底色與較大字體確保桌布不影響閱讀，選擇會保存。玻璃不支援時會明確提示並改用清晰深色。
- 清晰外觀不要求 Windows 11 的玻璃及圓角 API；Windows 10 可能使用直角外框。目前沒有 Windows 10 實機驗證，不能將建置成功視為所有 Windows 版本相容。
- 未配對裝置不能讀取文字或檔案；驗證碼／QR 邀請 5 分鐘到期、一次使用。驗證碼採 SRP-6a，TLS 憑證與請求装置身分一併驗證；不在 mDNS 廣播配對碼或 Room 秘密。

## 線上更新

Windows 1.0.5 起，每次啟動會在背景檢查 GitHub 新版。出現提示後按「是，更新」，程式下載並驗證簽章，接著關閉並啟動覆蓋更新，完成後重新開啟。也可在「連線與外觀設定」按「檢查更新」。按「稍後」不會下載或安裝。

舊版請先手動安裝 1.0.5 一次，以後不用手動下載安裝包或解除安裝。這是完整程式包的原地更新，不是不中斷執行的熱更新或差分更新。更新期間 Room 暫時離線；配對、設定、資料與未分享文字草稿保留。請先完成所有裝置的檔案傳輸。

只有版本檢查與更新包下載會連到 GitHub；共享文字、檔案與配對仍在 LAN。沒有網際網路時可以照常分享，更新失敗可稍後手動重試。Android 1.0.2 起也支援啟動檢查與「裝置 → 檢查 App 更新」。先手動覆蓋安裝本版一次，之後可在 App 內下載 APK，經 Android 系統確認後更新。首次可能需要允許 PocketDrop 安裝 App；不會繞過系統確認。

發布流程與簽章金鑰管理見 [docs/UPDATES.md](docs/UPDATES.md)。

## 從原有版本更新

Windows 保留原本的應用程式識別及本機資料路徑；Android 保留套件識別與發佈簽章，更新可沿用既有配對。請先關閉舊版 Windows PocketDrop，再啟動安裝後的版本，避免同一個 Room 同時啟動兩份。

## 開發

需要 Node.js、Rust MSVC、Visual Studio C++ Build Tools、WebView2；Android 需要 JDK 17、Gradle 8.11.1 與 Android SDK 35。

```powershell
npm ci
npm run desktop
npm run build
npm run test:rust
npm run package
```

Windows 安裝包輸出於 `src-tauri/target/release/bundle/nsis/`。

Android：設定 `ANDROID_HOME`、`JAVA_HOME`，以 Gradle 執行 `assembleRelease lintRelease testReleaseUnitTest`。正式更新簽章由本機環境提供，不包含在 Git 中；其他開發者應透過 `POCKETDROP_KEYSTORE`、`POCKETDROP_STORE_PASSWORD`、`POCKETDROP_KEY_ALIAS`、`POCKETDROP_KEY_PASSWORD` 指定自己的簽章。自行簽署的 APK 無法直接覆蓋官方 APK。

原始 Windows／Android 實驗記錄保留在 `docs/`，它們描述歷史版本。目前架構請看 [ARCHITECTURE_v1.0.md](docs/ARCHITECTURE_v1.0.md)，發佈檢查請看 [RELEASE_1.0.1.md](docs/RELEASE_1.0.1.md)。

## Daily-Agent 介接 API

Windows PocketDrop 1.0.5 可透過配對後的 HTTPS API 連接 Daily-Agent，提供共享文字讀寫與檔案列表。[安裝與 API 文件](integrations/daily-agent/README.md)。
