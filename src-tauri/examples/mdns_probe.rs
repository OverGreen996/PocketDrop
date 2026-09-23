//! Independent same-LAN probe. Same-host success is not a two-PC acceptance pass.
use mdns_sd::{IfKind, ServiceDaemon, ServiceEvent, ServiceInfo};
use std::{
    net::{IpAddr, Ipv4Addr, UdpSocket},
    time::{Duration, Instant},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address = std::env::args()
        .nth(1)
        .ok_or("Usage: mdns_probe <local LAN IPv4>")?;
    let ip: Ipv4Addr = address.parse()?;
    if !(ip.is_private() || ip.is_link_local())
        || !if_addrs::get_if_addrs()?
            .iter()
            .any(|i| i.ip() == IpAddr::V4(ip))
    {
        return Err("Use an assigned local LAN IPv4 address".into());
    }
    let socket = UdpSocket::bind((ip, 0))?;
    let daemon = ServiceDaemon::new()?;
    daemon.disable_interface(IfKind::All)?;
    daemon.enable_interface(IfKind::Addr(IpAddr::V4(ip)))?;
    let id = uuid::Uuid::new_v4().to_string();
    let props = [
        ("app", "PocketDrop"),
        ("pv", "0"),
        ("id", id.as_str()),
        ("cap", "discovery-only"),
    ];
    let service = "_pocketdrop-m0._udp.local.";
    let info = ServiceInfo::new(
        service,
        &format!("probe-{}", &id[..8]),
        &format!("probe-{id}.local."),
        address.as_str(),
        socket.local_addr()?.port(),
        &props[..],
    )?;
    let fullname = info.get_fullname().to_owned();
    let receiver = daemon.browse(service)?;
    daemon.register(info)?;
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut found = false;
    while Instant::now() < deadline {
        if let Ok(ServiceEvent::ServiceResolved(peer)) =
            receiver.recv_timeout(Duration::from_millis(500))
        {
            let peer_id = peer.get_property_val_str("id").unwrap_or_default();
            if peer_id != id
                && uuid::Uuid::parse_str(peer_id).is_ok()
                && peer.get_property_val_str("pv") == Some("0")
            {
                println!("{{\"event\":\"peer_resolved\",\"protocol\":0,\"authenticated\":false}}");
                found = true;
            }
        }
    }
    daemon
        .unregister(&fullname)?
        .recv_timeout(Duration::from_secs(2))?;
    daemon.shutdown()?.recv_timeout(Duration::from_secs(2))?;
    if !found {
        return Err("No other test instance resolved in 20 seconds".into());
    }
    Ok(())
}
