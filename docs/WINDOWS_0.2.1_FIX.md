# Windows 0.2.1 修正記錄

日期：2026-09-24。使用者回報多出系統標題列、右上方形邊角，以及檔案無法拖進 Shared Files。

> 更正：後續使用者截圖證實 0.2.1 仍有原生玻璃矩形底層。下方當時「四角沒有方形殘邊」的判定不成立；以 [0.2.2 修正與驗證](FIX_0.2.2.md) 為準。

## 根因與修改

1. `tauri-runtime-wry 2.11.4` 對 WindowContent webview 產生的是 `SynthesizedWindowEvent::DragDrop`。舊版只使用 `Builder::on_webview_event`，未收到主視窗拖放事件。現在於主視窗 `on_window_event` 統一處理 Enter／Over／Leave／Drop；Drop 只處理一次，背景執行本機登記、回饋成功數與拒絕原因。
2. `tao 0.35.3` 即使 decorations=false，頂層視窗仍保留 WS_CAPTION。圓角 region 與原生材質組合在焦點變更時會重新畫出傳統非客戶區。加入 UI 執行緒上的原生 subclass，過濾 caption／frame 樣式，WM_NCPAINT 不畫系統邊框；WM_NCACTIVATE 仍交給 Tao 處理，但 lParam=-1 阻止非客戶區重繪。
3. 圓角改在 UI 執行緒計算，依 Win32 視窗實際大小與前端 viewport 換算 CSS 28px。玻璃材質也在 UI 執行緒設定，避免工作執行緒 DPI 或 subclass 所屬執行緒不一致。

沒有變更配對格式、Room 信任資料、檔案 API 或 Android APK。不是以灰底或桌布截圖遮蓋視窗問題。

## 驗證

- TypeScript／Vite、Windows release build 成功。
- Rust 8 tests 通過。新增 Tauri MockRuntime 視窗拖放回歸：同次兩個檔案成功加入、資料夾提示 ZIP、phone-drop 正確回報、清單可用且不公開 local_path。
- 既有 13 組 HTTPS／WebSocket／檔案 API 情境通過。
- Windows 主機實際 UI：聚焦後沒有系統標題列，切換置頂後沒有復發，四角沒有方形殘邊。證據：`evidence/windows-0.2.1/frameless-pinned.jpg`。
- 本輪自動化工具不支援 Explorer → PocketDrop 的跨 App 拖曳；未宣稱已用實體滑鼠完成跨視窗拖放。請使用者以同一檔案重試。

## 安裝／重試

關閉舊版 → 完整解壓縮 `PocketDrop-Phone-Trial-0.2.1-windows-x64.zip` → 執行 `PocketDrop.exe` → 連上 Room 後拖入一個或多個本機檔案。預期顯示「放開以加入」，放開後顯示成功數與清單。使用一般使用者權限啟動即可。

APK 不需重裝。執行檔請放在固定解壓目錄，不要直接從 ZIP 暫存啟動；Windows 防火牆對不同執行檔路徑的允許狀態可能不同。
