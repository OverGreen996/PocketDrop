> 歷史版本文件。目前 1.0 架構請看 [ARCHITECTURE_v1.0.md](ARCHITECTURE_v1.0.md)。

# PocketDrop 架構 v0.1 — M0 邊界

產品優先情境：**Android 手機 ↔ Windows 電腦**。PC ↔ PC 只是前期驗證載具，不能取代手機分享文字、Share Target 加入檔案、手機隨選下載的產品驗收。M0 不先開發 Android，後續協定不可綁定 Windows 路徑或 Tauri IPC。

## 本輪實作

Windows x64 → Tauri 2（Rust）→ WebView2（TypeScript + CSS）。主窗透明、無標題列、固定 440 × 740 DIP。原生 `CreateRoundRectRgn/SetWindowRgn` 裁切 28 DIP 圓角，DPI 變化重新套用。標誌區可拖移，文字與按鈕不在拖移區域。

`window-vibrancy 0.6` 的 `apply_acrylic` 向 Windows compositor 請求背景 Acrylic。Tint 起點 RGBA(22,30,45,58)，另有 23% CSS 表面 tint、細亮邊與微高光。CSS 不使用 `backdrop-filter`，不截取桌布，不以貼圖模擬即時穿透。內卡只疊加半透明表面，共用原生背景模糊；未宣稱每張卡具有獨立原生 blur。Windows API 不提供跨版本一致的 CSS blur 半徑／飽和度旋鈕。

可關閉 Acrylic 做同一位置 A/B 對照；目前深／亮文字手動切換，尚未自動適應桌布。原生圓角使用 window region，需評估不同 Windows build 對合成與效能的影響。

## 拖放

檔案總管 → Tauri native DragDrop → Rust metadata 工作執行緒 → `drop-result` → DOM。

- 不靠瀏覽器的 `File.path`，也不讓前端提交任意路徑要求讀取。
- 只讀 metadata，不讀檔案內容、不計算 hash、不複製、不上傳。
- 每次最多 100 個、畫面最多 100 筆，避免大量項目耗盡 UI。
- 只傳 `name`、`size` 到畫面；完整路徑只短暫存在 Rust 拖放 callback 中。
- 拒絕資料夾、符號連結、UNC/device namespace，錯誤訊息不附帶完整路徑。
- 原生 WebView2/Tauri 自身仍可產生本機拖放事件；不將任何路徑送往網路。
- 接受任意副檔名；MIME 與檔案內容驗證屬未來真正傳輸功能。

## mDNS 探索 probe

使用 `mdns-sd`，服務型別 `_pocketdrop-m0._udp.local.`，與未來正式協定隔離。預設停止；只在使用者指定的本機 LAN IPv4 上啟動，先排除全部介面，再啟用所選位址。僅接受 RFC1918／link-local 位址；這不等同 Windows Private profile，VPN 也可能符合，測試者必須選信任的實體 LAN。

TXT allowlist：`app=PocketDrop`、`pv=0`、`id=<ephemeral UUID>`、`cap=discovery-only`。SRV 公開本機 OS 分配的 UDP port，A 公開所選 LAN 地址。UDP socket 只保留測試埠、不讀封包、不回應應用資料。沒有 HTTP、HTTPS、WebSocket listener，也沒有秘密、文字或檔案 metadata 廣播。

自己的 UUID 過濾掉；同機第二個 process 仍是測試 peer，**不能當作第二台 PC 的證據**。未驗證的 peer 使用 textContent 顯示、最多 100 筆。停止／退出時 unregister、stop browse、shutdown。正常離線依 goodbye/TTL，突然斷電可能延遲；M0 沒有心跳或可靠可用狀態。IP 改變後需停止、重開 App 並重新選介面，完整自動恢復不屬於本輪。

## 權限與資料

Tauri capability 限定主窗，僅需基本事件、拖移、關閉、最小化、置頂。CSP 不允許遠端腳本、遠端內容或應用中的 internet fetch。外部背景測試板是本機 static HTML，無私密資料。文字示意只在記憶體；複製由使用者點擊觸發。

Rust 拖放 log 是 JSON 計數，不記錄路徑、檔名、文字內容。尚未實作持久化 log/輪替。WebView2 自己的 runtime profile/cache 仍可能寫入 Windows app data；不要在 M0 貼入秘密資料。

## 後續方向（未實作，必須先通過 M0）

每裝置 client + local server，mDNS 僅發現，TLS + 身分釘選負責驗證；QR 的公開金鑰、短效 token 與初始端點用於建立信任。未配對設備不能讀資料。M1 若測試文字同步，需先界定開發配對與加密權限邊界，不能等 M4 才修補裸露資料服務。

SQLite 分開存 Room、Device、TextItem、FileItem、Event 與 LocalFileBinding。LocalFileBinding 永不序列化到網路。事件以 device sequence/Lamport 等確定性順序處理 LWW，不只比較牆上時間；刪除使用 tombstone，重連以增量事件補齊。檔案只同步 metadata，下載才串流拉取，支援 cancel/Range 與成熟 hash。這些都是設計約束，並未在 M0 實作。

桌面常駐、置頂、自動隱藏預留為後續視窗策略；本輪僅一般視窗＋置頂，未做 WorkerW 桌面掛載、tray、收合或邊緣喚出。
