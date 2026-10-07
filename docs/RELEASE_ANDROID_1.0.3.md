# PocketDrop Android 1.0.3 — 尚未發布

配合 Windows 重整深色配色、文字編輯卡片與主要分享按鈕，分頁加入選取狀態及輔助閱讀標示。既有配對、系統分享與傳輸流程不變。

目前缺少原 APK 的簽章金鑰，尚不能發布可覆蓋 1.0.2 的 APK，也未修改 OTA manifest。原簽章 SHA-256：1b44b5384101836bff52967d99f6711ea77bc532d5f6ec01bc82d7d2e5680438。請恢復金鑰後使用既有 package-android-release.ps1 驗證與發布；不要以不同簽章替代。

已以本機測試簽章完成 assembleRelease、lintRelease、testReleaseUnitTest；測試用 APK 不發布，也不更新 OTA。尚未實測實體 Android 手機。
