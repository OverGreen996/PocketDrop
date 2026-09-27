# PocketDrop 1.0.4 — 修正窄視窗文字區無法展開

## 問題與修正

1.0.3 在 CSS 視窗寬度不超過 360px 時，舊有手機尺寸樣式使用 `height:65px!important`，覆蓋文字自適應計算。重現結果：計算高度 481px、實際只有 65px。

移除該固定高度、最小高度及縮小字體的覆蓋規則。窄視窗與一般視窗統一使用自適應高度；仍保留前 1,000 字的展開計算上限，超出內容不截斷，可捲動與框選。

## 驗證

- 使用已建置前端與 headless Edge，覆蓋 320、360、361、440、1000 CSS px 寬度，以及深色、淺色、玻璃樣式，共 15 組。
- 每組檢查文字區真正展開、遵守最大高度、全文保留、框選不被重排清除，以及刪短後回到 180px。
- 測試程式：`scripts/test-text-layout.mjs`。對 1.0.3 的 320px 情境重現失敗；修正版通過。
- TypeScript／Vite 與 Windows NSIS 建置通過。此次未改原生視窗與傳輸協定；沒有新增 Windows 10 實機驗證。

## 安裝

關閉舊版後直接覆蓋安裝 PocketDrop-1.0.4-Windows-x64-Setup.exe。配對與資料沿用，Android 不需更新。

[Android 1.0.1 APK](https://github.com/OverGreen996/PocketDrop/releases/download/v1.0.1/PocketDrop-1.0.1-Android.apk)
