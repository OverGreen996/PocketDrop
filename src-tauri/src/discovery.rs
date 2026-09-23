use mdns_sd::{DaemonEvent, IfKind, ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::Serialize;
use std::{
    net::{IpAddr, Ipv4Addr, UdpSocket},
    sync::Mutex,
    time::Duration,
};
use tauri::Emitter;

const SERVICE: &str = "_pocketdrop-m0._udp.local.";
pub struct Discovery(pub Mutex<Option<Session>>);
pub struct Session {
    daemon: ServiceDaemon,
    fullname: String,
    _socket: UdpSocket,
}
impl Drop for Session {
    fn drop(&mut self) {
        if let Ok(done) = self.daemon.unregister(&self.fullname) {
            let _ = done.recv_timeout(Duration::from_secs(1));
        }
        let _ = self.daemon.stop_browse(SERVICE);
        let _ = self.daemon.shutdown();
    }
}

#[derive(Serialize)]
pub struct Interface {
    name: String,
    address: String,
}
fn allowed(ip: Ipv4Addr) -> bool {
    ip.is_private() || ip.is_link_local()
}
#[tauri::command]
pub fn interfaces() -> Result<Vec<Interface>, String> {
    let list = if_addrs::get_if_addrs().map_err(|_| "無法列出網路介面")?;
    Ok(list
        .into_iter()
        .filter_map(|i| match i.ip() {
            IpAddr::V4(ip) if allowed(ip) && !i.is_loopback() => Some(Interface {
                name: i.name,
                address: ip.to_string(),
            }),
            _ => None,
        })
        .collect())
}

#[derive(Clone, Serialize)]
struct PeerEvent {
    kind: String,
    session: String,
    id: String,
    fullname: String,
    addresses: Vec<String>,
    port: u16,
}

#[tauri::command]
pub async fn start_discovery(
    app: tauri::AppHandle,
    state: tauri::State<'_, Discovery>,
    address: String,
) -> Result<String, String> {
    let mut state = state.0.lock().map_err(|_| "Discovery lock failed")?;
    if state.is_some() {
        return Err("探索已啟動，請先停止".into());
    }
    let ip: Ipv4Addr = address.parse().map_err(|_| "無效介面")?;
    if !allowed(ip) || !interfaces()?.iter().any(|i| i.address == address) {
        return Err("僅可使用本機 LAN IPv4 介面".into());
    }
    // Reserve a real ephemeral UDP port; no content endpoint, no reads or replies.
    let socket = UdpSocket::bind((ip, 0)).map_err(|_| "無法使用此 LAN 介面")?;
    let port = socket.local_addr().map_err(|_| "無法取得測試埠")?.port();
    let id = uuid::Uuid::new_v4().to_string();
    let daemon = ServiceDaemon::new().map_err(|e| e.to_string())?;
    let result = (|| {
        daemon
            .disable_interface(IfKind::All)
            .map_err(|e| e.to_string())?;
        daemon
            .enable_interface(IfKind::Addr(IpAddr::V4(ip)))
            .map_err(|e| e.to_string())?;
        let props = [
            ("app", "PocketDrop"),
            ("pv", "0"),
            ("id", id.as_str()),
            ("cap", "discovery-only"),
        ];
        let info = ServiceInfo::new(
            SERVICE,
            &format!("pd-{}", &id[..8]),
            &format!("pd-{id}.local."),
            address.as_str(),
            port,
            &props[..],
        )
        .map_err(|e| e.to_string())?;
        let fullname = info.get_fullname().to_string();
        let monitor = daemon.monitor().map_err(|e| e.to_string())?;
        let events = daemon.browse(SERVICE).map_err(|e| e.to_string())?;
        daemon.register(info).map_err(|e| e.to_string())?;
        let handle = app.clone();
        std::thread::spawn(move || {
            while let Ok(event) = monitor.recv() {
                if matches!(event, DaemonEvent::Error(_)) {
                    let _ = handle.emit("m0-error", "mDNS 網路錯誤：請停止探索並檢查網路／防火牆");
                }
            }
        });
        let self_id = id.clone();
        std::thread::spawn(move || {
            while let Ok(event) = events.recv() {
                let peer = match event {
                    ServiceEvent::ServiceResolved(info) => {
                        let peer_id = info.get_property_val_str("id").unwrap_or_default();
                        if peer_id == self_id
                            || uuid::Uuid::parse_str(peer_id).is_err()
                            || info.get_property_val_str("pv") != Some("0")
                            || info.get_property_val_str("cap") != Some("discovery-only")
                        {
                            continue;
                        }
                        PeerEvent {
                            kind: "resolved".into(),
                            session: self_id.clone(),
                            id: peer_id.into(),
                            fullname: info.get_fullname().into(),
                            addresses: info.get_addresses().iter().map(|a| a.to_string()).collect(),
                            port: info.get_port(),
                        }
                    }
                    ServiceEvent::ServiceRemoved(_, fullname) => PeerEvent {
                        kind: "removed".into(),
                        session: self_id.clone(),
                        id: String::new(),
                        fullname,
                        addresses: vec![],
                        port: 0,
                    },
                    _ => continue,
                };
                let _ = app.emit("discovery-peer", peer);
            }
        });
        Ok(fullname)
    })();
    match result {
        Ok(fullname) => {
            *state = Some(Session {
                daemon,
                fullname,
                _socket: socket,
            });
            Ok(id)
        }
        Err(error) => {
            let _ = daemon.shutdown();
            Err(error)
        }
    }
}

#[tauri::command]
pub async fn stop_discovery(state: tauri::State<'_, Discovery>) -> Result<(), String> {
    state.0.lock().map_err(|_| "Discovery lock failed")?.take();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scope_excludes_public_and_loopback() {
        for ip in ["127.0.0.1", "8.8.8.8", "0.0.0.0"] {
            assert!(!allowed(ip.parse().unwrap()));
        }
        for ip in ["192.168.1.2", "10.0.0.2", "172.16.2.2", "169.254.1.2"] {
            assert!(allowed(ip.parse().unwrap()));
        }
    }
}
