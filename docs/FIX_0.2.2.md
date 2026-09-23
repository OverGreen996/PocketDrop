# PocketDrop 0.2.2 修正與驗證

2026-09-24。使用者確認 Android 配對仍存在，但關閉重開後顯示離線；Windows 則仍有玻璃矩形底層超出 CSS 圓角。

## Windows 整層圓角

0.2.1 的 HRGN 只裁到了視窗內容，沒有裁掉 DWM 的 Acrylic 底層。本次也實測了 accent Acrylic + HRGN，完整背景板截圖仍有矩形，故不採用。

最終改用 Windows 11 的 `DWMWA_WINDOW_CORNER_PREFERENCE / DWMWCP_ROUND`，移除與 DWM 圓角不相容的自訂 window region，搭配原生 `DWMSBT_TRANSIENTWINDOW`。整層背景由 DWM 裁切。外層 CSS 改為 8px，配合系統較小的圓角；內層卡片保留原本圓角。這與原提案建議的 24–32px 大圓角有差異，沒有假造大圓角已通過。

在 Windows build 26200 的高對比外部背景板上，確認整層矩形殘邊消失，背景色可穿透、條紋被原生模糊。證據：`evidence/windows-0.2.2/native-rounded-on-external-board.jpg`。這張截圖包含 Widget 邊界外的背景，避免只看內側的錯誤判定。保留無框、拖放與原有 Room。

目前此圓角 Acrylic 路徑需要 Windows 11 22H2 以上。Windows 10／舊 Windows 11 會在外觀設定明確顯示不支援，不宣稱通過；尚未提供同等效果。Windows 最大化、貼齊螢幕、遠端／VM、多 DPI 行為仍有 OS 限制。

依據：[Microsoft 原生圓角文件](https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/ui/apply-rounded-corners) 說明 window region 不相容，以及 [DWM System Backdrop](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwm_systembackdrop_type)。

## Android 重連

確認的程式缺陷包含：舊 NSD callback 可能停止新一輪探索；舊版解析中會丟掉其他 found 事件；狀態讀取與探索共用單一工作佇列；WebSocket 失敗會將成功的 HTTPS 連線錯誤標成離線。未直接取得使用者手機 Log，不將其中任一項宣稱為該手機唯一根因。

- App 回前景或重新開啟時，保留 Keystore／credential，建立新的連線週期；取消舊狀態請求與 WebSocket，立即嘗試已保存端點。
- 狀態讀取與探索驗證使用各自佇列；控制 API 總逾時 8 秒，不限制大檔傳輸總時間。
- Android 14+ 使用持續 NSD ServiceInfoCallback；Android 8–13 排隊解析並重試。每個 callback 綁定自己的探索週期，舊 callback 不得修改新連線。
- 探索取得的新位址需通過原有憑證 pin 與 Room 授權，才保存；不重新呼叫配對 API、不接受新的未知憑證。
- 配對及重新發現的持久化在 UI 執行緒串行提交，避免舊探索覆寫新 credential。失敗保留配對並顯示原因。
- WebSocket 中斷會重建；線上／離線以成功的 HTTPS 狀態為準。

API 參考：[Android NsdManager](https://developer.android.com/reference/android/net/nsd/NsdManager)。此版仍需要手機在前景，沒有背景常駐服務。

## 驗證

- Windows release、TypeScript/Vite 成功；Rust 9 個測試通過（含 DWM 圓角／移除不相容 region、檔案拖放回歸）。
- 13 組獨立 HTTPS／WebSocket／檔案 API 情境通過。
- Android assembleDebug、lintDebug、testDebugUnitTest 成功；6 個測試通過。新增 4 項使用正式 RoomClient 的實際 TLS/HTTP 測試：新 client 載入已保存 JSON 後存取狀態而不配對、PC 連接埠改變後保留信任、拒絕錯誤憑證、取消卡住的請求後重連。
- APK 簽章與 0.2 相同，applicationId 不變，versionCode 從 2 升到 3。可覆蓋安裝保留資料。
- 未接上使用者實體 Android。上述測試未覆蓋 Android Keystore 的實際程序重啟、OEM NSD、使用者 Wi-Fi 環境；需依下列步驟確認。

## 升級驗收

1. Windows 使用 0.2.2 EXE；Android 直接安裝 0.2.2 APK 覆蓋舊版，不解除安裝、不清資料、不按忘記 Room。
2. 電腦保持開啟，同一 Wi-Fi 下開啟手機 App。先不掃 QR，確認顯示「已沿用原有配對重新連線」。
3. 手機從最近使用清單滑掉 App，再開啟，連續測試三次，確認能讀文字與檔案清單。
4. 再重啟 PC（端點可改變），手機保持前景等自動尋找；預期 5–30 秒恢復，路由器封鎖 mDNS 時不保證。
5. 若還失敗，記下手機頂端的明確原因，保留配對，不以一直重掃 QR 掩蓋重連問題。
