> 歷史版本文件。目前 1.0 架構請看 [ARCHITECTURE_v1.0.md](ARCHITECTURE_v1.0.md)。

# Android ↔ Windows 實機驗收

## 0.2.2 最新結果

Windows 原生整層圓角已在 build 26200 外部高對比背景上驗證；0.2.1 的大圓角矩形殘邊判定有誤，以下舊版敘述不能作為通過證據。新版採系統較小圓角，Win11 22H2+ 支援完整材質。Rust 9 個、Android 6 個（含 4 項真實 TLS 重連）測試通過，13 個 API 情境通過。Android APK 更新為 0.2.2／versionCode 3，簽章與原版相同。

使用者已在實體手機完成過配對，但回報重開 App 離線。此修正版仍等待同一手機驗證重開自動重連。先覆蓋安裝，不解除安裝、不忘記 Room、不重掃 QR。詳細步驟與測試邊界見 [FIX_0.2.2.md](FIX_0.2.2.md)。

## 已自動驗證

- TypeScript + Vite release build。
- Windows 0.2.1：Rust 8 個測試；新增主視窗 WindowEvent 拖放的多檔加入／資料夾拒絕／UI 回饋回歸測試，以及原生邊框樣式保留／移除測試。
- Android `assembleDebug lintDebug testDebugUnitTest`：APK 建置與 Java 邏輯測試；Lint 0 errors，仍有已註明的依賴版本／自訂 pin 驗證器／文字國際化警告。
- `scripts/phone-smoke.mjs`：獨立 Windows loopback HTTPS 程序的 13 組情境，詳見 `evidence/phone-trial/api-smoke.json`。包含 2 MiB 隨機檔案逐 byte 比對、空檔、中文檔名、來源刪除、惡意路徑、大小驗證、WebSocket 授權／refresh 與測試客戶端的 TLS pin 不符拒絕。
- 實際 Windows QR 畫面截圖已透過 `scripts/verify-qr-screenshot.mjs` 成功解碼並檢查必要欄位；已修正高 DPI QR 裁切。結果在 `evidence/phone-trial/qr-result.json`。截图邀請在關閉測試 App 後失效，不供使用者實際配對。

**上述不是兩台實體裝置測試。尚未連接 Android 手機或模擬器，APK 能安裝、相機掃码、NSD、防火牆、Share Target 與 MediaStore 行為需實機確認。** 不以編譯成功代替使用成功。

Windows 0.2.1 實際畫面已確認聚焦、切換置頂時沒有額外系統標題列，原生圓角与 CSS 外層對齊；截圖見 `evidence/windows-0.2.1/frameless-pinned.jpg`。原 M0 的系統標題列缺陷已修正；其他桌布／Win10／多螢幕驗收仍待完成。拖放修正已通過 Tauri MockRuntime WindowEvent → 共享清單的回歸測試，但本輪自動化工具不能操作 Explorer 到另一個 App 的跨視窗拖曳，實際 Explorer 拖放仍需使用者重試。

APK 沿用 0.2，已使用 Android SDK apksigner 驗證 v2 簽章；套件 `local.pocketdrop.android`、min SDK 26、target SDK 35，為 debug 簽章。

## 兩台裝置測試順序

| 測項 | 操作 | 通過標準 | 實機結果 |
|---|---|---|---|
| 安裝 | 手機自行複製 APK，點開安裝 | 能進入文字／檔案／裝置頁 | 待驗收 |
| 第一次配對 | 電腦「配對手機」，手機「裝置→掃描」 | 電腦列出手機，手機顯示已連線 | 待驗收 |
| PC → 手機文字 | PC 貼入中文、URL、多行 Prompt，按分享 | 手機即時更新，複製後內容一致 | 待驗收 |
| 手機 → PC 文字 | 手機輸入並分享，或瀏覽器分享文字至 PocketDrop | PC 顯示同一份文字 | 待驗收 |
| 電腦檔案 | 拖入 txt + STL + ZIP，手機只看清單 | 看到檔名／大小，未下載前手機沒有副本 | 待驗收 |
| 下載 | 手機點其中檔案下載 | 有進度、速度、完成提示，檔案可取用 | 待驗收 |
| 手機上傳 | 加入手機檔案；相簿分享一／多張至 PocketDrop | PC 清單更新，「下載/PocketDrop」內有原始檔案 | 待驗收 |
| 大檔取消 | 選足够大的影片，開始傳輸後按取消 | 停止網路傳送，未完成檔案不公開；能再傳其他檔案 | 待驗收 |
| 來源變更 | PC 拖入後移動／刪除來源檔案 | 手機顯示目前無法取得、下載不返回錯誤內容 | 待驗收 |
| 電腦離線 | 關閉 PC App | 手機 5–30 秒內顯示離線（依網路 timeout） | 待驗收 |
| 重新上線 | 重開 PC App、手機 App 保持前景 | 自動找回、保留配對，文字／清單恢復 | 待驗收 |
| 配對持久化 | 兩端關閉重開 | 不需重掃 QR；內容仍在 | 待驗收 |
| 移除裝置 | 電腦按手機旁「移除裝置」 | 手機無法再讀寫，重掃新邀請才能加入 | 待驗收 |
| 邀請重放 | 再掃已用的 QR；或等超過 5 分鐘 | 配對拒絕 | 自動測試涵蓋重放，實機待驗收 |
| 玻璃 | 移到深／亮／多色真實桌布 | 反映外部顏色且文字可讀 | M0 實際桌布項目仍待驗收 |

## 連不上時

1. 確認電腦與手機為同一 LAN；手機不要只用行動數據。
2. 關閉路由器訪客裝置隔離，或改用正常家庭 Wi-Fi；不需要開路由器 port forward。
3. 允許 PocketDrop 使用 Windows 私人網路。不要整個停用防火牆。
4. 若電腦有 VPN／虛擬網卡，在 Widget「連線與外觀設定」選實際 Wi-Fi／乙太網路後按「套用」，再產生 QR。
5. 重開後連不上：先使用 0.2.2 的「重新尋找電腦」，保留配對並記下頂端失敗原因。只有電腦確實撤銷／更換配對才重新掃 QR；普通離線不應要求重配。手機日期明顯錯誤可能導致 TLS 憑證時間檢查失敗。
6. 手機分享的來源若沒有提供檔案大小，先存到手機內部儲存再分享。本版不自動下載第三方雲端 URL。

回報時提供手機型號、Android 版本、PC Windows 版本、哪一步失敗和畫面錯誤文字。不要提供配對 QR、credential、私鑰或完整私人文字內容。
