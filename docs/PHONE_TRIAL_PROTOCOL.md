> 歷史版本文件。目前 1.0 架構請看 [ARCHITECTURE_v1.0.md](ARCHITECTURE_v1.0.md)。

# Phone Trial v0.2 協定與界線

這是經使用者要求增加的 Android ↔ Windows 實用性測試分支，不是 M4/M5 正式協定完成宣告。PC 為此版本唯一 Room 持有者，Android 為前景客戶端；沒有雲端、UPnP、路由器映射或網際網路探索。

## 身分、信任與通道

PC 第一次產生持久化 UUID 與 rcgen EC TLS 憑證，私鑰使用 Windows 當前使用者 DPAPI 加密保存。Android 產生 Keystore EC 身分金鑰及 UUID。手機身分公鑰隨配對登記；**此版存取認證是 TLS 上的每裝置隨機 bearer credential，尚未實作公鑰簽名挑戰／mTLS**。不要宣稱已完成最終裝置簽名協定。

QR 包含 app、protocol_version=1、mode=phone-trial、room_id、PC device_id、公開金鑰、憑證 SHA-256、256-bit 隨機邀請及初次 LAN HTTPS 端點。PC 同時只保留一個 5 分鐘邀請；產生新邀請替換舊的，成功配對即消耗。邀請和資料 API 都不允許明文 HTTP。

Android 只接受 HTTPS 私有／link-local IPv4 literal 端點，拒絕公網、DNS 主機、userinfo、額外路徑或 query；不跟隨 HTTP redirect。TLS 使用 OkHttp／平台 TLS，檢查憑證有效期及 QR 釘選的完整憑證 SHA-256。自訂 TrustManager 是為了配對後的自簽憑證，並非 trust-all；Android Lint 的 custom trust manager 警告保留供審查。

每個 credential 是獨立 CSPRNG 256-bit 值。PC 資料庫僅存 SHA-256；Android 以 Keystore AES-GCM 加密存在本機 preferences，關閉備份與裝置轉移。QR 不持久化，credential 不写入 log。Room ID 只是範圍檢查，不充當密碼。具有正確 credential 及 Room ID 的成員才能讀写。

PC 可從 Widget 撤銷裝置。新請求立即拒絕；下載／上傳每個區塊檢查權限；WebSocket 在變更或最多 2 秒心跳時關閉。已送入作業系統緩衝區的資料無法追回。正式版仍需要更完整的撤銷事件與多節點信任傳播。

## 探索、重連、同步

PC 綁定選定 LAN IPv4 與隨機埠，透過 `_pocketdrop._tcp.local.` 公告 app、pv、device id、cap。沒有內容、路徑、憑證秘密、邀請 token。Android NSD 解析已保存 PC ID；新位址必須通過原憑證 pin 和授權 state 請求才採用，無法只靠偽造 mDNS 接管。

前景 App 建立 WebSocket，變更通知觸發 state 重讀，5 秒輪詢作保險。重新連線讀完整快照，內容以 PC SQLite 為準；離線手機不能發佈新的 Room 狀態。文字以 PC transaction 遞增序號排序，每次修改留最近 20 筆歷史。檔案刪除使用 SQLite deleted 標記。這不能替代最終多裝置事件日誌、Lamport LWW 與離線合併。

Android 文字／清單只有記憶體快取，App 程序重開且 PC 離線時不顯示先前清單；只有配對信任資料會在 Android 持久化。

## API

| 方法／路徑 | 用途 | 驗證 |
|---|---|---|
| POST /v1/pair | 消耗 QR 邀請、建立手機 credential | 短效一次性邀請 |
| GET /v1/state | 文字、序號、有效檔案、成員快照 | Bearer + x-pocketdrop-room |
| PUT /v1/text | 更新目前文字，UTF-8 最多 32 KiB | 同上 |
| GET /v1/events | WebSocket refresh 提示 | 同上 |
| PUT /v1/files?name=… | 手機串流上傳，x-file-size 必填 | 同上 |
| GET /v1/files/{uuid} | 串流下載、單段 HTTP Range | 同上 |

移除裝置、移除檔案、產生邀請僅透過本機 Tauri command，不公開管理 HTTP 路由。未配對者會收到 401；Room 不符 403；來源變更 410；無效 range 416。

## 檔案與本機資料

FileView 只含 UUID、檔名、大小、來源標籤、可用性與可選 SHA-256。本機路徑只存在 bindings 資料表。來源檔案大小／mtime 變更後不可下載，請重新拖入；符號連結、UNC、資料夾、Windows 保留檔名與路徑穿越拒絕。所有格式視為一般二進位資料。

PC 拖入只登記 metadata，不複製檔案。Android 上傳寫入「下載/PocketDrop/隨機 UUID/檔名」，先用隨機 partial 檔案串流，收到精確長度才更名並發布。取消或錯誤清除 partial（強制終止程序可能留下暫存檔／空資料夾，沒有自動清理排程）。下載每次最多讀 64 KiB；手機不使用整檔 RAM。SHA-256 在 PC 背景計算，下載時有 hash 才驗證；若尚未計算完成，手機明確標示尚未比對。TLS 仍驗證传輸通道。

Android 10+ 使用 MediaStore Downloads/IS_PENDING，未完成檔案不公開；取消／驗證失敗刪除，完成才發布。Android 8–9 使用 App 專用目錄與 FileProvider。開啟檔案需要使用者按鈕，執行檔類型不由 App 啟動。

SQLite 文字與接收檔案本身不加密，依賴使用者的 OS 帳號與磁碟保護；本版沒有 at-rest 全資料庫加密保證。日誌不寫文字／金鑰／本機路徑；錯誤訊息對遠端只回傳固定代碼。

## 尚待完成

正式多裝置 Room、手機來源直取、Android 背景生命週期、傳輸續傳 UI、歷史 UI、可設定接收目錄、裝置在線狀態、主機 IP 變更自動重新綁定、大檔／壓力／長時間測試、安裝包與發行簽章、完整安全審查。Windows 原生玻璃的 OS 支援差異仍依 M0 驗收記錄。
