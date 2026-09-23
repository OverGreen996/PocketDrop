> 歷史版本文件。目前 1.0 架構請看 [ARCHITECTURE_v1.0.md](ARCHITECTURE_v1.0.md)。

# Android ↔ Windows 可用性測試版

使用者 2026-09-23 明確要求手機版，並確認 Android、可安裝 APK。這次授權擴大到兩端可用性驗證，取代上一輪「停在 M0 不做手機」的停止點。產品主軸仍是手機 ↔ 電腦。

本輪交付：Android APK、Windows EXE、QR 短效配對、持久化信任／TLS pin、mDNS 重找 PC、雙向文字、PC 原生拖放後手機按需下載、Android 選檔／Share Target 上傳 PC、進度／取消、撤銷裝置。

這是明示的 phone-trial 協定，不冒充完成原規格：Room 資料由這台 PC 保存，手機新增檔案會主動上傳此 PC；PC 關閉即不可用。手機前景保持同步，背景不提供常駐伺服器。去中心化、Android 原始檔來源服務、跨 PC Room、歷史設定與完整衝突合併仍待後續。原始需求文件與 M0 證據保留。

安全底線：只綁定所選本機 LAN IPv4；HTTPS；QR 釘選憑證；一次性 5 分鐘隨機 token；未授權 API 不得讀文字／檔案；每台手機独立可撤銷 bearer credential（PC 只保存 hash，Android Keystore 加密保存）；不將本機路徑序列化；成熟 TLS／AES-GCM 函式庫；不開 UPnP／port forwarding／cloud endpoint。

驗證：Rust 邊界測試＋真實 HTTPS 整合測試、Android build/lint/APK 簽章與 manifest 檢查。除非有已連接裝置，不能宣稱手機硬體掃描／Share Target 已實測。
