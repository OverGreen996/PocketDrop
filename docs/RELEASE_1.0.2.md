# PocketDrop 1.0.2 — Windows 清晰外觀

- 新增「清晰深色」「清晰淺色」，預設清晰深色。背景完全不透明，桌布不影響文字對比。
- 放大主要文字、按鈕及提示文字。右上角 ◐ 直接開啟外觀設定。
- 外觀保存於本機，重開時恢復；原生玻璃仍可選用。
- 原生玻璃不支援時提示原因並保存清晰外觀，不再停留在難以閱讀的半透明背景。
- 關閉原生背景與更新裝飾圓角不再要求 Windows 11 API 成功。

## 安裝

下載 PocketDrop-1.0.2-Windows-x64-Setup.exe，關閉舊版後直接覆蓋安裝，不需移除配對。
Android 沒有變更，繼續使用 [1.0.1 APK](https://github.com/OverGreen996/PocketDrop/releases/download/v1.0.1/PocketDrop-1.0.1-Android.apk)。

## 驗證與限制

- TypeScript / Vite 建置及 Windows NSIS 打包通過。
- 原生視窗測試 2 項通過。
- 外觀控制器模擬檢查：預設深色、保存淺色的載入、玻璃成功、玻璃不支援時降級與保存、無效設定恢復預設，均通過。
- 深／淺色的主要文字、提示文字，在主底色、卡片、輸入底色上計算對比度最低 6.74:1。
- Windows 11 原生視窗確認深色、淺色及右上角外觀捷徑。重新啟動後 UI 自動化工具回報 foreground window did not report a process id，因此不宣稱已完成重啟畫面驗證；保存與讀取由上述控制器檢查覆蓋。
- Windows 10／11 x64 為目標；清晰外觀不依賴 Acrylic。不承諾 Windows 7／8 或所有歷史版本支援。沒有 Windows 10 實機，尚未完成其實機驗證。
- 玻璃仍需要 Windows 11 22H2 以上；Windows 10 外框可能保持直角。
- Windows 安裝包尚無 Authenticode 簽章。沿用現有 WebView2 安裝方式、應用程式識別和 Room 資料；未改變網路及 Android 協定。
