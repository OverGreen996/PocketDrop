# PocketDrop

區域網路裡的共享暫存盒。文字貼進去，檔案丟進去，自己的裝置就能取用。

**Windows + Android · 1.0.1**

## 下載與安裝

到 [Releases](https://github.com/OverGreen996/PocketDrop/releases/latest) 下載：

- **PocketDrop-1.0.1-Windows-x64-Setup.exe**：Windows 安裝包，安裝於目前使用者帳號。
- **PocketDrop-1.0.1-Android.apk**：複製到 Android 手機後點擊安裝。已有 PocketDrop 時直接覆蓋更新，請勿先解除安裝。
- `SHA256SUMS.txt`：下載檔案校驗值。

Windows 建議使用 **Windows 11 22H2 或更新版本、x64**。Android 需要 **Android 8.0 以上**。
Windows 安裝包尚未使用 Authenticode 簽章；Microsoft WebView2 尚未安裝時，安裝程序需要網路下載其官方執行環境。PocketDrop 的文字、檔案與配對只走 LAN。

## 開始使用

1. 在一台常開的電腦啟動 PocketDrop。首次啟動會準備自己的 Room。
2. 按 **邀請裝置**，畫面會顯示電腦編號、8 位驗證碼及 QR Code。
3. 第二台電腦按 **加入既有 Room**；Android 到 **裝置 → 輸入驗證碼加入**，選擇相同編號並輸入驗證碼。也可掃描 QR；沒有攝影機的電腦可選取 QR 圖片。
4. 所有裝置使用同一個 Wi-Fi／LAN。配對完成後會保存身分，重新開啟、換 IP 或連接埠改變時自動尋找原有電腦，不用重掃。
5. 貼上文字後按 **分享文字**。將檔案直接拖進 Windows Widget；其他裝置按 **下載** 才取得檔案。

Windows 防火牆提示是讓自己的裝置能在私人網路連線；請允許信任的私人網路。訪客 Wi-Fi 的裝置隔離可能阻止互相發現。

## 這一版的運作方式

**建立 Room 的電腦必須保持開啟。** 這是目前版本採用的運作方式，尚未提供無中央裝置的 Room。

- Windows 電腦拖入的檔案只登錄資訊，不預先複製。下載時由 Room 電腦轉送來源電腦的串流，不落地保存中繼副本。
- Android 分享／加入檔案會上傳至 Room 電腦保存；接收裝置仍按需下載。
- 電腦下載位置為 `下載/PocketDrop`。Android 10 以上存入 `Downloads/PocketDrop`；Android 8–9 存入 App 專用下載資料夾。
- 來源電腦離線後約 12 秒內顯示不可用；重新上線會恢復。已移除紀錄不會被來源重新宣告而復活。
- 每台來源電腦最多 200 筆檔案；單檔最多 16 GiB，每次拖入最多 100 個。不支援資料夾，請先壓縮成 ZIP。
- 支援進度、取消、串流、SHA-256、HTTP Range；尚未提供下載中斷後的自動續傳介面。
- Android 開啟 App 後連線；不提供背景常駐同步服務。
- 真實背景 Acrylic 使用 Windows compositor。Windows 10 不保證玻璃與圓角效果，失敗會在外觀設定顯示，不用灰底冒充。
- 未配對裝置不能讀取文字或檔案；驗證碼／QR 邀請 5 分鐘到期、一次使用。驗證碼採 SRP-6a，TLS 憑證與請求装置身分一併驗證；不在 mDNS 廣播配對碼或 Room 秘密。

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
