//! Exercises the real updater's version check and signature verification, never installation.
use axum::{extract::State, routing::get, Json, Router};
use std::sync::{Arc, RwLock};
use tauri_plugin_updater::UpdaterExt;

#[tokio::test]
#[ignore = "Set PD_UPDATE_PACKAGE to a signed Windows installer, then run --ignored update_tests"]
async fn signed_download_rejects_tampering_and_skips_old_versions() {
    let path = std::env::var("PD_UPDATE_PACKAGE").expect("signed installer path");
    let bytes = std::fs::read(&path).unwrap();
    let signature = std::fs::read_to_string(format!("{path}.sig")).unwrap();
    let production: serde_json::Value =
        serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let config = production["plugins"]["updater"].clone();
    assert!(config["endpoints"][0]
        .as_str()
        .unwrap()
        .starts_with("https://"));
    assert_ne!(config["dangerousInsecureTransportProtocol"], true);
    assert_ne!(config["allowDowngrades"], true);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let manifest = Arc::new(RwLock::new(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "platforms": { "windows-x86_64": {"url": format!("{base}/package"), "signature": signature.trim()} }
    })));
    let package = Arc::new(RwLock::new(bytes.clone()));
    let manifest_state = manifest.clone();
    let package_state = package.clone();
    let router =
        Router::new()
            .route(
                "/latest",
                get(
                    |State(data): State<Arc<RwLock<serde_json::Value>>>| async move {
                        Json(data.read().unwrap().clone())
                    },
                )
                .with_state(manifest_state),
            )
            .route(
                "/package",
                get(|State(data): State<Arc<RwLock<Vec<u8>>>>| async move {
                    data.read().unwrap().clone()
                })
                .with_state(package_state),
            );
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let mut context = tauri::test::mock_context(tauri::test::noop_assets());
    context.package_info_mut().version = "1.0.4".parse().unwrap();
    let mut test_config = config;
    // Only this loopback fixture permits HTTP; production always uses HTTPS.
    test_config["dangerousInsecureTransportProtocol"] = true.into();
    test_config["endpoints"] = serde_json::json!([format!("{base}/latest")]);
    context
        .config_mut()
        .plugins
        .0
        .insert("updater".into(), test_config);
    let app = tauri::test::mock_builder()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .build(context)
        .unwrap();
    let updater = app
        .updater_builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
    let update = updater.check().await.unwrap().unwrap();
    let mut received = 0;
    let verified = update.download(|n, _| received += n, || {}).await.unwrap();
    assert_eq!(verified, bytes);
    assert_eq!(received, bytes.len());
    package.write().unwrap()[100] ^= 1;
    assert!(
        update.download(|_, _| {}, || {}).await.is_err(),
        "tampered executable must be rejected"
    );
    for version in ["1.0.4", "1.0.3"] {
        manifest.write().unwrap()["version"] = version.into();
        assert!(updater.check().await.unwrap().is_none());
    }
    *package.write().unwrap() = bytes;
    manifest.write().unwrap()["version"] = "99.0.0".into();
    let replay = updater.check().await.unwrap().unwrap();
    assert!(
        replay.download(|_, _| {}, || {}).await.is_err(),
        "signed version must match manifest version"
    );
    manifest.write().unwrap()["version"] = "invalid".into();
    assert!(updater.check().await.is_err());
    server.abort();
    assert!(updater.check().await.is_err());
}

#[tokio::test]
#[ignore = "Uses the public GitHub release endpoint; downloads but never installs"]
async fn published_update_download_verifies() {
    let production: serde_json::Value =
        serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let mut context = tauri::test::mock_context(tauri::test::noop_assets());
    context.package_info_mut().version = "1.0.4".parse().unwrap();
    context
        .config_mut()
        .plugins
        .0
        .insert("updater".into(), production["plugins"]["updater"].clone());
    let app = tauri::test::mock_builder()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .build(context)
        .unwrap();
    let updater = app
        .updater_builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .unwrap();
    let update = updater.check().await.unwrap().unwrap();
    assert_eq!(update.version, env!("CARGO_PKG_VERSION"));
    let bytes = update.download(|_, _| {}, || {}).await.unwrap();
    assert!(bytes.len() > 1000000);
    assert_eq!(&bytes[..2], b"MZ");
}
