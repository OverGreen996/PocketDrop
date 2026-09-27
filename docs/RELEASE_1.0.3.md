# PocketDrop 1.0.3 — 可縮放視窗與文字自適應高度

- 拖曳視窗四邊或四角可自由調整大小，最小 400 × 480；保留無標題列外觀。
- Shared Text 根據實際文字與換行自動增高，視窗寬度變化時重新排版。
- 自動展開最多採計前 1,000 個 Unicode 字元，高度上限為視窗高度 65% 或 600 px 中較小者，下限 180 px。
- 超過上限時文字區內捲動，內容不截斷，仍可框選、Ctrl+A 與複製。原有 32,768 輸入長度限制不變。
- 縮放操作區只在外緣，不覆蓋文字選取區。清晰深色、清晰淺色與玻璃皆保留。
- 目前重新開啟仍使用預設視窗大小，不保存視窗尺寸。

## 安裝

關閉舊版後直接安裝 PocketDrop-1.0.3-Windows-x64-Setup.exe，不必解除安裝或重新配對。
Android 沒有變更，繼續使用 [1.0.1 APK](https://github.com/OverGreen996/PocketDrop/releases/download/v1.0.1/PocketDrop-1.0.1-Android.apk)。

## 驗證

- TypeScript / Vite 建置與 Windows NSIS 安裝包建置通過。
- 原生視窗 2 項測試通過，包含保留縮放樣式與 compositor 圓角。
- 原有外觀控制器檢查通過。
- 以已建置前端在 headless Edge 驗證：短文字 180 px、長文字 481 px；超過 1,000 字仍保存全文且高度不再增長；視窗從 440 加寬至 1,000 px 後測試文字高度由 481 降至 253 px。框選範圍在重排後保留、Ctrl+A 全選、刪短後縮回、八個縮放操作區均通過。
- Windows UI 自動化工具回報 foreground window did not report a process id，未能完成新版實機拖曳縮放檢查。沒有 Windows 10 實機，亦不宣稱已完成其實機驗證。
- Windows 安裝包未使用 Authenticode 簽章；玻璃仍需要 Windows 11 22H2 以上，Windows 10 建議使用清晰外觀。
