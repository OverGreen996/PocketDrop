# PocketDrop ↔ Daily-Agent API 介接

適用 Windows、Node.js 24+，PocketDrop 1.0.5 即可使用，無須重裝 PocketDrop。以 Daily-Agent `codex/initial-release`（`8b5e0ff`）的 ToolBroker 介面實作。此模組只在使用者明確呼叫工具時連線，不會背景監看或把 Room 文字自動送入模型。

## 安裝與配對

1. 將 PocketDrop 原始碼下載到固定資料夾，在本資料夾執行 `npm ci --ignore-scripts`。
2. 在**建立 Room 的電腦**開啟 PocketDrop → 邀請裝置，將完整 QR Code 截圖存成 PNG。邀請五分鐘有效、一次使用；驗證碼不能當 API Token。
3. 執行 `node pair.mjs "C:\你的資料夾\邀請.png"`。成功後刪除截圖。Daily-Agent 會出現在 PocketDrop 的裝置列表。
4. 停止 Daily-Agent，執行 `node install.mjs "C:\你的資料夾\Daily-Agent\daily-agent"`，然後重新啟動 Daily-Agent。

安裝器會備份 `tools/ToolBroker.js.before-pocketdrop`，僅在已知結構吻合時修改。保留本模組位置：安裝器使用絕對模組路徑。Daily-Agent 更新覆蓋 ToolBroker 時需重新安裝介接；若結構改變會停止而不修改。還原備份即可卸載，並在 PocketDrop 移除 Daily-Agent 裝置以撤銷存取。

可以對電腦上的 Daily-Agent 說：

- 「讀取 PocketDrop 的共享文字。」
- 「把這段文字分享到 PocketDrop：明天十點開會。」
- 「列出 PocketDrop 的檔案。」

寫入會取代目前共享文字並同步至 Room。工具輸出會交给 Daily-Agent 處理；使用雲端模型時，主動讀取的內容可能送往該模型，請依自己的 Daily-Agent 模型設定使用。

## 程式介面

```js
import { PocketDropClient } from './client.mjs';
import { loadProfile } from './profile.mjs';
const client = new PocketDropClient(await loadProfile());
const snapshot = await client.state(); // text, revision, files 等
await client.writeText('由 Daily-Agent 分享');
```

配對憑證存於 `%LOCALAPPDATA%\PocketDrop\daily-agent.dpapi`，用目前 Windows 使用者的 DPAPI 加密。不儲存邀請 Token，不記錄共享文字，不把憑證放進 Git。整合使用既有 Room 成員權限，並非細分權限的服務帳號；持有憑證者具備一般成員 API 權限。

### 既有 HTTPS v1 API

除了首次配對，請求需要 `Authorization: Bearer <credential>` 與 `x-pocketdrop-room: <room_id>`。TLS 使用邀請提供的 SHA-256 憑證指紋釘選；不可使用一般的「忽略 TLS 錯誤」HTTP 客戶端。

| Method | Path | 說明 |
| --- | --- | --- |
| POST | `/v1/pair` | 一次性邀請配對；token、device_id、name、public_key |
| GET | `/v1/state` | Room 文字、revision、檔案 metadata、裝置；不含來源本機路徑 |
| PUT | `/v1/text` | JSON `{ "content": "文字" }`；UTF-8 上限 32768 bytes；成功 204 |
| GET | `/v1/events` | 已驗證 WebSocket refresh 通知，再讀 state |
| PUT | `/v1/files?name=<URL編碼檔名>` | 串流 binary，`x-file-size` 指定長度；成功 201 |
| GET | `/v1/files/{file_id}` | 隨選下載，支援 HTTP Range |

401 表示未配對／已撤銷，403 Room 不符，413 內容過大。檔案下載另有 410 來源不可用、416 Range 無效。API 不是公開網站，沒有新增 CORS、網際網路入口或 Cloudflare 路由。

## 範圍與限制

- 本次 Daily-Agent 工具提供文字讀寫與檔案列表；未加入 AI 自動下載、上傳或執行檔案。
- 僅電腦使用者互動可呼叫；Daily-Agent 手機／遠端橋接與背景事件無法呼叫這些工具。PocketDrop 自己的 Android 同步仍照常運作。
- Room 電腦需要開啟，兩端同一 LAN。端點失效時透過 mDNS 找相同 Device ID，並再次驗證 TLS 指紋與 Room；不使用固定 IP。不對可能已成功的寫入自動重試。
- 此 SDK 支援 IPv4 私有網段與 loopback；多網卡、跨子網、mDNS 被防火牆封鎖可能無法找到裝置。
- API 文字目前是最後寫入覆蓋，沒有 compare-and-swap；請避免多個自動化同時改寫。
- 以真實 Windows PocketDrop 執行檔驗證配對、文字往返、撤銷與 TLS 錯誤；Daily-Agent 工具接入另有測試。未宣稱已實測使用者的模型對話或跨電腦 LAN。

驗證：設定 `PD_INTEROP_EXE` 指向 PocketDrop 執行檔後執行 `npm test`；未設定時略過實際 API 測試。
