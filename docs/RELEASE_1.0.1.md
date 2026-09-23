# PocketDrop 1.0.1 — 電腦重開後的自動重連修正

## 問題與原因

1.0.0 的 Windows 服務每次啟動隨機選取新連線埠，但使用相同 mDNS service 與 hostname。手機保存的端點因此過期；當 Android 解析到舊快取或第一次探測失敗後未再收到更新，便可能一直顯示離線。原本的 Java 重啟測試直接讀取新 bootstrap 端點後呼叫 `at(newEndpoint)`，只驗證了「已知新位置後可沿用配對」，沒有證明自動找回位置。

## 修正

- Windows 保存上次成功選到的連線埠，重啟時優先沿用；被占用時才改用新的可用埠。不固定 IP、不綁定公網介面。
- 每次啟動 listener 使用新的 mDNS service instance 和 hostname，讓重新出現的服務不沿用舊 SRV／A 快取。Device ID、Room ID、TLS 憑證及配對權杖保持不變。
- Android 保存最近發現的候選位置，離線時每 5 秒重試；不再必須等 NSD 重新送出相同端點的回呼。候選有容量、有效期限與併發限制，只有通過原 TLS pin 和 Room 授權才會保存新位置。
- 連線從在線轉為離線時立刻重啟探索並關閉舊 WebSocket。NSD 新版 service callback 註冊失敗時回退到 resolve 流程。
- 仍保留前景／背景世代檢查；舊回呼不能覆寫新配對。

## 回歸檢查

- 新 Java 重啟測試先對 1.0.0 EXE 執行，因原連線位置改變而失敗，確認能抓到舊版缺口。
- Windows 真實 mDNS／HTTPS 檢查：占住舊連線埠、重新建立同一個 Room 服務；只透過 mDNS 找候選位置，不從 QR 或 bootstrap 注入新端點，驗證文字與原配對可恢復。
- Android Java 用戶端對新版 Windows EXE：連續停止／啟動服務 3 次，原有用戶端以及從原保存資料重建的用戶端都必須恢復；不呼叫 `at(newEndpoint)`、不重新配對。
- 候選排程檢查涵蓋：沒有新 NSD 回呼仍會重試、優先較新的連線埠、併發／期限／清除限制，以及拒絕公網候選。

測試使用開發機上的 Windows 服務與正式 Android Java 網路程式碼，不等同 Android 實機 NSD／各路由器的完整驗收。平台與簽章限制沿用 [1.0.0 發佈紀錄](RELEASE_1.0.0.md)。

本次結果：Rust 13 項、Android 10 項（包含三次真實 Windows 服務重啟）、HTTPS API 13 項全部通過；Windows 安裝包及 Android Release APK 建置、Lint 均成功。

## 更新

Windows 安裝包與 Android APK 都更新至 1.0.1。Android versionCode 5，沿用原簽章。關閉舊版電腦端後覆蓋安裝，手機直接覆蓋更新；不要解除安裝、清除 App 資料或移除既有配對。建立 Room 的電腦仍需開啟。

參考：[Android NSD callback 官方文件](https://developer.android.com/reference/android/net/nsd/NsdManager.ServiceInfoCallback)。
