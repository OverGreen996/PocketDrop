# PocketDrop 1.0 架構

本版採使用者確認的 PC-hosted Room。Windows 既可建立 Room，也可加入另一台 Windows 的 Room；Android 為前景連線用戶端。沒有雲端、帳號、UPnP、路由器轉送或網際網路穿透。

## 傳輸與發現

每台 Windows 啟動限定私有 IPv4 介面的 HTTPS 本機服務，使用持久化自簽 TLS 憑證。Windows 私鑰及加入 Room 的權杖使用 DPAPI 保存；Android 使用 Keystore 保護配對。`_pocketdrop._tcp.local.` 僅公告 app、protocol version、device ID、port、能力。mDNS 只是端點來源，每次重連仍須通過原憑證 pin 與 Room 權限。禁止代理及重新導向，拒絕公網端點。

Windows 用戶端每 2 秒讀取最新狀態並登錄來源 metadata。Android 使用 WebSocket 提示及 5 秒恢復輪詢。Windows 偵測本機介面／IP 改變後重新綁定；用戶端透過相同 Device ID 及憑證重新定位。Android NSD 有世代隔離，舊回呼不能覆蓋新連線。

## 配對

QR 包含協定版本、Room／Device ID、公鑰、TLS leaf SHA-256、一次性 256-bit token 及私有連線端點。為保持既有 APK 相容，JSON 的 legacy `mode` 仍使用 `phone-trial`；這不是產品介面或版本標籤。

手動配對為 8 位隨機碼，使用 SRP-6a / RFC 5054 2048-bit 群組及 SHA-256；RustCrypto SRP 執行群組計算，proof 使用 Bouncy Castle SRP6Client 的標準定長編碼。SRP identity 同時綁定服務端 Device ID、實際握手所見 TLS leaf hash、加入裝置 ID／名稱／公鑰。Bootstrap 的未信任 TLS 用戶端只能讀取公開 hello，不帶任何權杖／文字／檔案；後續連線 pin 到其 leaf，SRP proof 驗證完成才能保存信任。伺服器 5 分鐘到期、每個邀請最多 5 次開始嘗試、只保留一個待完成交換。QR 與驗證碼共用同一份一次性邀請。

SRP 證明與測試不等同獨立安全稽核；本次未進行第三方稽核。

## 檔案

完整路徑只在來源 Windows 的 SQLite bindings。Room metadata 不含路徑。

來源 PC 的 authenticated `/v1/source` 登錄檔案清單、來源 TLS pin 與限本 Room 電腦使用的回讀憑證，這些敏感回讀資料在主機資料庫以 DPAPI 加密。Room 對外只提供隨機 file ID 和一般檔案資訊。對來源 PC 的讀取按需透過 Room 串流轉送；不預先複製、不緩存整個檔案。來源最後一次登錄超過 12 秒便不可用。來源裝置被撤銷時，主機拒絕其來源紀錄及串流；接收端被撤銷也會中斷串流。

Android Share Target / 系統選檔以串流上傳至 Room 電腦。所有下載用檔名驗證、唯一子目錄／MediaStore、長度檢查、取消清理；不自動執行接收檔案。SHA-256 在來源背景計算，完成後隨回應提供；雜湊未完成時不宣稱已比對。支援單一 HTTP Range。刪除 remote_ids tombstone 不會因來源再次登錄消失。

## 文字與儲存

Room 電腦以交易序號決定文字版本，不靠客戶端時鐘。保存最近 20 次變更。用戶端重連取得完整目前狀態，保留尚未分享的文字草稿。尚未提供歷史瀏覽、歷史數量設定、多主機衝突合併或在主機離線時繼續共享。

## Windows 外觀

Tauri 2、透明無框 WebView2、DWM DWMSBT_TRANSIENTWINDOW 真實原生背景材質。以 DWM 圓角及一致的外層 HTML 半徑消除矩形底層；不用 SetWindowRgn 強制大圓角（這會破壞 DWM 圓角）。內層卡片共用原生模糊，疊加輕 tint 和高光。Windows 11 22H2 以上為完整外觀支援目標；舊版 Windows 顯示不支援原因，不偽造桌布。
