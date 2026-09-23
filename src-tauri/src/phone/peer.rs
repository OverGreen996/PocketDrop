//! Windows Room client. Profiles are DPAPI protected; mDNS is only a locator.
use super::*;
use reqwest::Client;
use rustls::{
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    pki_types::{CertificateDer, ServerName, UnixTime},
    DigitallySignedStruct, SignatureScheme,
};
#[derive(Debug)]
struct Pin(String);
impl ServerCertVerifier for Pin {
    fn verify_server_cert(
        &self,
        end: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: &ServerName<'_>,
        _: &[u8],
        _: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        if hash(end.as_ref()) == self.0 {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::General("paired certificate mismatch".into()))
        }
    }
    fn verify_tls12_signature(
        &self,
        m: &[u8],
        c: &CertificateDer<'_>,
        d: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            m,
            c,
            d,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }
    fn verify_tls13_signature(
        &self,
        m: &[u8],
        c: &CertificateDer<'_>,
        d: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            m,
            c,
            d,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}
pub fn endpoint(s: &str) -> Result<String, String> {
    let u = reqwest::Url::parse(s).map_err(|_| "無效連線資料")?;
    let ip = u
        .host_str()
        .and_then(|h| h.parse::<Ipv4Addr>().ok())
        .ok_or("僅允許 LAN 位址")?;
    if u.scheme() != "https"
        || !local_ip(ip)
        || !u.username().is_empty()
        || u.password().is_some()
        || u.query().is_some()
        || u.fragment().is_some()
        || u.path() != "/"
        || u.port_or_known_default() == Some(0)
    {
        return Err("僅允許 LAN 安全連線".into());
    }
    Ok(s.trim_end_matches('/').into())
}
pub fn http(pin: &str) -> Result<Client, String> {
    if pin.len() != 64 || !pin.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("無效裝置身分".into());
    }
    let _ = rustls::crypto::ring::default_provider().install_default();
    let tls = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(Pin(pin.to_owned())))
        .with_no_client_auth();
    Client::builder()
        .use_preconfigured_tls(tls)
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(4))
        .read_timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Profile {
    pub endpoint: String,
    pub certificate_sha256: String,
    pub credential: String,
    pub room_id: String,
    pub device_id: String,
}
impl Profile {
    pub fn request(
        &self,
        method: reqwest::Method,
        path: &str,
    ) -> Result<reqwest::RequestBuilder, String> {
        endpoint(&self.endpoint)?;
        Ok(http(&self.certificate_sha256)?
            .request(method, format!("{}{path}", self.endpoint))
            .bearer_auth(&self.credential)
            .header("x-pocketdrop-room", &self.room_id))
    }
    pub async fn snapshot(&self) -> Result<Snapshot, String> {
        let response = self
            .request(reqwest::Method::GET, "/v1/state")?
            .timeout(Duration::from_secs(6))
            .send()
            .await
            .map_err(|_| "正在重新尋找電腦，配對已保存")?;
        let state: Snapshot = checked(response)
            .await?
            .json()
            .await
            .map_err(|_| "Room 回應不正確")?;
        if state.room_id != self.room_id || state.device_id != self.device_id {
            return Err("Room 身分不符".into());
        }
        Ok(state)
    }
}
pub async fn checked(response: reqwest::Response) -> Result<reqwest::Response, String> {
    if response.status().is_success() {
        Ok(response)
    } else {
        Err(match response.status().as_u16() {
            401 => "配對已撤銷或邀請已失效",
            429 => "嘗試次數過多，請產生新邀請",
            404 | 410 | 503 => "目前無法取得，來源装置已離線或檔案已變更",
            _ => "操作未完成，請稍後重試",
        }
        .into())
    }
}
pub fn save(c: &Core, p: &Profile) -> Result<(), String> {
    let mut current = c.joined.lock().unwrap();
    persist(c, p)?;
    *current = Some(p.clone());
    Ok(())
}
fn persist(c: &Core, p: &Profile) -> Result<(), String> {
    use std::io::Write;
    let data = protect(&serde_json::to_vec(p).map_err(|e| e.to_string())?, false)?;
    let temporary = PartialFile(c.dir.join(format!(".joined-{}.tmp", uuid::Uuid::new_v4())));
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary.0)
        .map_err(|_| "無法保存配對")?;
    file.write_all(&data)
        .and_then(|_| file.sync_all())
        .map_err(|_| "無法保存配對")?;
    drop(file);
    std::fs::rename(&temporary.0, c.dir.join("joined-room.dpapi")).map_err(|_| "無法保存配對")?;
    Ok(())
}
fn relocate(c: &Core, old: &Profile, next: &Profile) -> bool {
    let mut current = c.joined.lock().unwrap();
    if current
        .as_ref()
        .is_none_or(|p| p.credential != old.credential)
    {
        return false;
    }
    if persist(c, next).is_err() {
        return false;
    }
    *current = Some(next.clone());
    true
}
pub fn load(dir: &FsPath) -> Result<Option<Profile>, String> {
    let p = dir.join("joined-room.dpapi");
    if !p.exists() {
        return Ok(None);
    }
    let bytes = protect(&std::fs::read(p).map_err(|_| "無法讀取配對")?, true)?;
    let profile = serde_json::from_slice(&bytes).map_err(|_| "已保存的配對損毀，請還原備份")?;
    Ok(Some(profile))
}
#[derive(Clone, Serialize)]
pub struct NearbyRoom {
    pub device_id: String,
    pub endpoint: String,
    pub name: String,
}
#[tauri::command]
pub async fn nearby_rooms(state: tauri::State<'_, Phone>) -> Result<Vec<NearbyRoom>, String> {
    let c = current(&state)?;
    let rooms = c
        .nearby
        .lock()
        .unwrap()
        .values()
        .filter(|(_, at)| at.elapsed() < Duration::from_secs(3600))
        .map(|(v, _)| v.clone())
        .collect();
    Ok(rooms)
}
pub fn discover(c: &Arc<Core>) -> Result<(), String> {
    let daemon = ServiceDaemon::new().map_err(|e| e.to_string())?;
    let rx = daemon.browse(SERVICE).map_err(|e| e.to_string())?;
    *c.browser.lock().unwrap() = Some(daemon);
    let weak = Arc::downgrade(c);
    std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            let Some(c) = weak.upgrade() else { break };
            if let mdns_sd::ServiceEvent::ServiceResolved(info) = event {
                let id = info.get_property_val_str("id").unwrap_or_default();
                if id == c.device_id
                    || uuid::Uuid::parse_str(id).is_err()
                    || info.get_property_val_str("app") != Some("PocketDrop")
                    || info.get_property_val_str("pv") != Some("1")
                {
                    continue;
                }
                for address in info.get_addresses() {
                    if let Ok(ip) = address.to_string().parse::<Ipv4Addr>() {
                        if local_ip(ip) {
                            let ep = format!("https://{ip}:{}", info.get_port());
                            let mut map = c.nearby.lock().unwrap();
                            if map.len() >= 128 {
                                map.retain(|_, (_, t)| t.elapsed() < Duration::from_secs(60));
                            }
                            if map.len() < 128 || map.contains_key(&ep) {
                                map.insert(
                                    ep.clone(),
                                    (
                                        NearbyRoom {
                                            device_id: id.into(),
                                            endpoint: ep,
                                            name: format!("PocketDrop · {}", &id[..8]),
                                        },
                                        Instant::now(),
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        }
    });
    Ok(())
}
#[tauri::command]
pub async fn join_qr(state: tauri::State<'_, Phone>, qr: String) -> Result<(), String> {
    qr_pair(&current(&state)?, qr).await
}
pub async fn qr_pair(c: &Arc<Core>, qr: String) -> Result<(), String> {
    if qr.len() > 4096 {
        return Err("邀請資料過大".into());
    }
    let p: serde_json::Value = serde_json::from_str(&qr).map_err(|_| "無法讀取 QR Code")?;
    if p["app"] != "PocketDrop" || p["protocol_version"] != 1 {
        return Err("不是 PocketDrop 邀請".into());
    }
    let s = |k: &str| {
        p[k].as_str()
            .map(str::to_string)
            .ok_or("邀請資料不完整".to_string())
    };
    let endpoint = endpoint(&s("endpoint")?)?;
    let pin = s("certificate_sha256")?;
    let response = http(&pin)?
        .post(format!("{endpoint}/v1/pair"))
        .timeout(Duration::from_secs(8))
        .json(&PairInput {
            token: s("token")?,
            device_id: c.device_id.clone(),
            name: device_name(),
            public_key: c.public_key.clone(),
        })
        .send()
        .await
        .map_err(|_| "無法連上邀請電腦")?;
    let result: PairOutput = checked(response)
        .await?
        .json()
        .await
        .map_err(|_| "配對回應無效")?;
    if result.room_id != s("room_id")? || result.device_id != s("device_id")? {
        return Err("邀請身分不符".into());
    }
    complete(&c, endpoint, pin, result).await
}
fn device_name() -> String {
    std::env::var("COMPUTERNAME")
        .unwrap_or("Windows 電腦".into())
        .chars()
        .filter(|c| !c.is_control())
        .take(60)
        .collect()
}
#[tauri::command]
pub async fn join_code(
    state: tauri::State<'_, Phone>,
    address: String,
    code: String,
) -> Result<(), String> {
    let c = current(&state)?;
    code_pair(&c, address, code).await
}
pub async fn code_pair(c: &Arc<Core>, address: String, code: String) -> Result<(), String> {
    let endpoint = endpoint(&address)?;
    // Bootstrap has no credentials or shared content. SRP authenticates the exact
    // leaf certificate observed here before any profile can be persisted.
    let bootstrap = Client::builder()
        .danger_accept_invalid_certs(true)
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .tls_info(true)
        .timeout(Duration::from_secs(6))
        .build()
        .map_err(|e| e.to_string())?;
    let response = bootstrap
        .get(format!("{endpoint}/v1/hello"))
        .send()
        .await
        .map_err(|_| "無法連上邀請電腦")?;
    let cert = response
        .extensions()
        .get::<reqwest::tls::TlsInfo>()
        .and_then(|i| i.peer_certificate())
        .ok_or("無法驗證配對連線")?;
    let pin = hash(cert);
    let hello: serde_json::Value = response
        .json()
        .await
        .map_err(|_| "邀請電腦版本不支援驗證碼")?;
    let host = hello["device_id"].as_str().ok_or("邀請資料無效")?;
    uuid::Uuid::parse_str(host).map_err(|_| "邀請資料無效")?;
    let client =
        pairing::ClientProof::new(c.device_id.clone(), device_name(), c.public_key.clone());
    let http = http(&pin)?;
    let response = http
        .post(format!("{endpoint}/v1/code/start"))
        .timeout(Duration::from_secs(8))
        .json(&client.start)
        .send()
        .await
        .map_err(|_| "配對連線中斷")?;
    let ch: pairing::Challenge = checked(response)
        .await?
        .json()
        .await
        .map_err(|_| "配對回應無效")?;
    let (proof, expected) = client.respond(host, &pin, &code, &ch)?;
    let response = http
        .post(format!("{endpoint}/v1/code/finish"))
        .timeout(Duration::from_secs(8))
        .json(&proof)
        .send()
        .await
        .map_err(|_| "配對連線中斷")?;
    let result: pairing::Completed = checked(response)
        .await?
        .json()
        .await
        .map_err(|_| "配對回應無效")?;
    let actual = STANDARD.decode(result.proof).map_err(|_| "配對驗證失敗")?;
    if !bool::from(subtle::ConstantTimeEq::ct_eq(
        expected.as_slice(),
        actual.as_slice(),
    )) || result.paired.device_id != host
        || Some(result.paired.room_id.as_str()) != hello["room_id"].as_str()
    {
        return Err("驗證碼或電腦身分不符".into());
    }
    complete(c, endpoint, pin, result.paired).await
}
async fn complete(
    c: &Core,
    endpoint: String,
    certificate_sha256: String,
    result: PairOutput,
) -> Result<(), String> {
    if result.credential.len() != 43 {
        return Err("配對回應無效".into());
    }
    if result.device_id == c.device_id {
        return Err("不能加入自己的 Room".into());
    }
    let p = Profile {
        endpoint,
        certificate_sha256,
        credential: result.credential,
        room_id: result.room_id,
        device_id: result.device_id,
    };
    let snapshot = p.snapshot().await?;
    save(c, &p)?;
    *c.cached.lock().unwrap() = Some(snapshot);
    c.changed();
    Ok(())
}
#[tauri::command]
pub fn room_mode(state: tauri::State<'_, Phone>) -> Result<String, String> {
    Ok(if current(&state)?.joined.lock().unwrap().is_some() {
        "joined"
    } else {
        "host"
    }
    .into())
}
#[tauri::command]
pub fn leave_room(state: tauri::State<'_, Phone>) -> Result<(), String> {
    let c = current(&state)?;
    let mut joined = c.joined.lock().unwrap();
    let p = c.dir.join("joined-room.dpapi");
    if p.exists() {
        std::fs::remove_file(p).map_err(|_| "無法移除配對")?;
    }
    joined.take();
    c.cached.lock().unwrap().take();
    c.changed();
    Ok(())
}

/// Only metadata is registered. The owner's original file is read on demand.
#[derive(Clone, Serialize, Deserialize)]
pub struct Source {
    pub endpoint: String,
    pub certificate_sha256: String,
    pub credential: String,
    pub room_id: String,
    pub files: Vec<FileView>,
}
pub async fn source(
    State(c): State<Arc<Core>>,
    headers: HeaderMap,
    Json(s): Json<Source>,
) -> ApiResult<StatusCode> {
    let (owner, _) = c.authorize(&headers)?;
    endpoint(&s.endpoint).map_err(|_| (StatusCode::BAD_REQUEST, "invalid_endpoint"))?;
    if s.credential.len() != 43
        || s.certificate_sha256.len() != 64
        || !s.certificate_sha256.bytes().all(|b| b.is_ascii_hexdigit())
        || uuid::Uuid::parse_str(&s.room_id).is_err()
        || s.files.len() > 200
        || s.files.iter().any(|f| {
            uuid::Uuid::parse_str(&f.file_id).is_err()
                || !clean_name(&f.name)
                || f.size > MAX_FILE
                || f.sha256
                    .as_ref()
                    .is_some_and(|h| h.len() != 64 || !h.bytes().all(|b| b.is_ascii_hexdigit()))
        })
    {
        return Err((StatusCode::BAD_REQUEST, "invalid_metadata"));
    }
    let data = protect(&serde_json::to_vec(&s).map_err(internal)?, false).map_err(internal)?;
    let db = c.db.lock().map_err(internal)?;
    db.execute("INSERT INTO sources(owner,data,seen) VALUES(?1,?2,?3) ON CONFLICT(owner) DO UPDATE SET data=excluded.data,seen=excluded.seen",params![owner,data,now()]).map_err(internal)?;
    for f in &s.files {
        db.execute(
            "INSERT OR IGNORE INTO remote_ids(owner,source,id) VALUES(?1,?2,?3)",
            params![owner, f.file_id, uuid::Uuid::new_v4().to_string()],
        )
        .map_err(internal)?;
    }
    drop(db);
    c.changed();
    Ok(StatusCode::NO_CONTENT)
}
pub fn remote_files(c: &Core) -> Result<Vec<FileView>, String> {
    let db = c.db.lock().unwrap();
    let mut stmt=db.prepare("SELECT s.owner,s.data,s.seen FROM sources s JOIN members m ON m.id=s.owner WHERE m.revoked=0").map_err(|e|e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Vec<u8>>(1)?,
                r.get::<_, u64>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut out = vec![];
    for r in rows {
        let (owner, data, seen) = r.map_err(|e| e.to_string())?;
        let s: Source =
            serde_json::from_slice(&protect(&data, true)?).map_err(|e| e.to_string())?;
        for mut f in s.files {
            let id: Option<String> = db
                .query_row(
                    "SELECT id FROM remote_ids WHERE owner=?1 AND source=?2 AND deleted=0",
                    params![owner, f.file_id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            if let Some(id) = id {
                f.file_id = id;
                f.available &= now().saturating_sub(seen) < 12;
                f.origin = format!("電腦 {}", &owner[..8]);
                out.push(f);
            }
        }
    }
    Ok(out)
}
pub async fn remote_download(
    c: Arc<Core>,
    id: String,
    headers: HeaderMap,
    digest: String,
) -> ApiResult<Response> {
    let row = {
        let db = c.db.lock().map_err(internal)?;
        db.query_row("SELECT r.source,s.data,s.seen,s.owner FROM remote_ids r JOIN sources s ON s.owner=r.owner JOIN members m ON m.id=s.owner WHERE r.id=?1 AND r.deleted=0 AND m.revoked=0",[&id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,Vec<u8>>(1)?,r.get::<_,u64>(2)?,r.get::<_,String>(3)?))).optional().map_err(internal)?.ok_or((StatusCode::NOT_FOUND,"file_removed"))?
    };
    if now().saturating_sub(row.2) >= 12 {
        return Err((StatusCode::SERVICE_UNAVAILABLE, "source_offline"));
    }
    let source: Source =
        serde_json::from_slice(&protect(&row.1, true).map_err(internal)?).map_err(internal)?;
    let f = source
        .files
        .iter()
        .find(|f| f.file_id == row.0 && f.available)
        .ok_or((StatusCode::GONE, "source_unavailable"))?;
    let mut request = http(&source.certificate_sha256)
        .map_err(internal)?
        .get(format!(
            "{}/v1/files/{}",
            endpoint(&source.endpoint).map_err(internal)?,
            row.0
        ))
        .bearer_auth(&source.credential)
        .header("x-pocketdrop-room", source.room_id);
    let (_, count, partial) = byte_range(
        headers.get(header::RANGE).and_then(|h| h.to_str().ok()),
        f.size,
    )
    .map_err(|_| (StatusCode::RANGE_NOT_SATISFIABLE, "invalid_range"))?;
    if let Some(range) = headers.get(header::RANGE) {
        request = request.header(header::RANGE, range);
    }
    let response = request
        .send()
        .await
        .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "source_offline"))?;
    if response.status()
        != if partial {
            StatusCode::PARTIAL_CONTENT
        } else {
            StatusCode::OK
        }
        || response.content_length() != Some(count)
    {
        return Err((StatusCode::GONE, "source_changed"));
    }
    let mut builder = Response::builder().status(response.status());
    for key in [
        "content-length",
        "content-range",
        "accept-ranges",
        "x-content-sha256",
    ] {
        if let Some(v) = response.headers().get(key) {
            builder = builder.header(key, v);
        }
    }
    let stream = response.bytes_stream();
    let owner = row.3;
    let stream = futures_util::stream::try_unfold(
        (stream, c, digest, owner, count),
        |(mut stream, c, digest, owner, left)| async move {
            if !c.valid_digest(&digest)
                || !c
                    .db
                    .lock()
                    .map_err(|_| std::io::Error::other("database busy"))?
                    .query_row("SELECT revoked=0 FROM members WHERE id=?1", [&owner], |r| {
                        r.get::<_, bool>(0)
                    })
                    .unwrap_or(false)
            {
                return Err(std::io::Error::other("revoked"));
            }
            match stream.next().await {
                Some(Ok(b)) if b.len() as u64 <= left => {
                    let next = left - b.len() as u64;
                    Ok(Some((b, (stream, c, digest, owner, next))))
                }
                None if left == 0 => Ok(None),
                _ => Err(std::io::Error::other("incomplete source")),
            }
        },
    );
    builder
        .header("content-type", "application/octet-stream")
        .header("content-disposition", "attachment")
        .header("cache-control", "no-store")
        .body(Body::from_stream(stream))
        .map_err(internal)
}
pub async fn heartbeat(c: &Arc<Core>, p: &Profile) -> Result<(), String> {
    let credential = {
        let mut callback = c.callback.lock().unwrap();
        if callback.as_ref().is_none_or(|(id, _)| id != &p.device_id) {
            let token = random_token();
            c.db.lock().unwrap().execute("INSERT INTO members(id,name,token_hash,pubkey,revoked) VALUES(?1,'Room 電腦',?2,'callback',0) ON CONFLICT(id) DO UPDATE SET token_hash=excluded.token_hash,revoked=0",params![p.device_id,hash(token.as_bytes())]).map_err(|e|e.to_string())?;
            *callback = Some((p.device_id.clone(), token));
        }
        callback.as_ref().unwrap().1.clone()
    };
    let files = c.local_snapshot()?.files;
    let endpoint = c.endpoint.lock().unwrap().clone();
    let source = Source {
        endpoint,
        certificate_sha256: c.fingerprint.clone(),
        credential,
        room_id: c.room_id.clone(),
        files,
    };
    checked(
        p.request(reqwest::Method::PUT, "/v1/source")?
            .timeout(Duration::from_secs(6))
            .json(&source)
            .send()
            .await
            .map_err(|_| "連線中斷")?,
    )
    .await?;
    Ok(())
}
pub fn background(c: Arc<Core>, app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut tick = 0u64;
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            tick += 1;
            let p = c.joined.lock().unwrap().clone();
            if let Some(mut p) = p {
                let mut result = p.snapshot().await;
                if result.is_err() {
                    let candidates: Vec<_> = c
                        .nearby
                        .lock()
                        .unwrap()
                        .values()
                        .filter(|(v, _)| v.device_id == p.device_id)
                        .map(|(v, _)| v.endpoint.clone())
                        .take(8)
                        .collect();
                    for endpoint in candidates {
                        let mut candidate = p.clone();
                        candidate.endpoint = endpoint;
                        if let Ok(snapshot) = candidate.snapshot().await {
                            if relocate(&c, &p, &candidate) {
                                p = candidate;
                                result = Ok(snapshot);
                                break;
                            }
                        }
                    }
                }
                if c.joined
                    .lock()
                    .unwrap()
                    .as_ref()
                    .is_none_or(|current| current.credential != p.credential)
                {
                    continue;
                }
                match result {
                    Ok(snapshot) => {
                        *c.cached.lock().unwrap() = Some(snapshot);
                        let _ = heartbeat(&c, &p).await;
                        let _ = app.emit("room-online", true);
                    }
                    Err(_) => {
                        let _ = app.emit("room-online", false);
                    }
                }
            }
            // Rebind after DHCP/interface changes or a failed listener. Identity survives.
            if tick % 5 == 0 {
                let endpoint = c.endpoint.lock().unwrap().clone();
                let ip = reqwest::Url::parse(&endpoint)
                    .ok()
                    .and_then(|u| u.host_str()?.parse::<Ipv4Addr>().ok());
                let interfaces = if_addrs::get_if_addrs().unwrap_or_default();
                if !ip.is_some_and(|ip| interfaces.iter().any(|i| i.ip() == IpAddr::V4(ip))) {
                    if let Some(ip) = interfaces
                        .iter()
                        .filter_map(|i| match i.ip() {
                            IpAddr::V4(ip) if local_ip(ip) => Some(ip),
                            _ => None,
                        })
                        .next()
                    {
                        let _ = serve(c.clone(), ip, false).await;
                    }
                }
            }
            let _ = app.emit("phone-changed", ());
        }
    });
}

#[tauri::command]
pub async fn download_file(
    app: tauri::AppHandle,
    state: tauri::State<'_, Phone>,
    file_id: String,
) -> Result<String, String> {
    let c = current(&state)?;
    let joined = c.joined.lock().unwrap().clone();
    let p = if let Some(p) = joined {
        p
    } else {
        let credential = random_token();
        c.db.lock().unwrap().execute("INSERT INTO members(id,name,token_hash,pubkey,revoked) VALUES(?1,'本機',?2,'internal',0) ON CONFLICT(id) DO UPDATE SET token_hash=excluded.token_hash,revoked=0",params![c.device_id,hash(credential.as_bytes())]).map_err(|_|"無法建立本機下載")?;
        Profile {
            endpoint: c.endpoint.lock().unwrap().clone(),
            certificate_sha256: c.fingerprint.clone(),
            credential,
            room_id: c.room_id.clone(),
            device_id: c.device_id.clone(),
        }
    };
    let snapshot = p.snapshot().await?;
    let f = snapshot
        .files
        .iter()
        .find(|f| f.file_id == file_id && f.available)
        .ok_or("來源裝置離線")?;
    if !clean_name(&f.name) || f.size > MAX_FILE {
        return Err("不安全的檔案資訊".into());
    }
    let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let mut jobs = c.downloads.lock().unwrap();
        if !jobs.is_empty() {
            return Err("請等待下載完成或先取消".into());
        }
        jobs.insert(file_id.clone(), cancel.clone());
    }
    let result=async{let sub=c.inbox.join(uuid::Uuid::new_v4().to_string());tokio::fs::create_dir_all(&sub).await.map_err(|_|"無法建立下載資料夾")?;let partial=PartialFile(sub.join("download.part"));let path=sub.join(&f.name);
 let response=checked(p.request(reqwest::Method::GET,&format!("/v1/files/{file_id}"))?.send().await.map_err(|_|"下載連線中斷")?).await?;if response.content_length()!=Some(f.size){return Err("檔案大小不符".into())}let expected=response.headers().get("x-content-sha256").and_then(|h|h.to_str().ok()).map(str::to_owned).or_else(||f.sha256.clone());
 let mut data=response.bytes_stream();let mut out=tokio::fs::File::create(&partial.0).await.map_err(|_|"無法寫入下載")?;let mut done=0;let start=Instant::now();let mut last=Instant::now();let mut hash=Sha256::new();loop{if cancel.load(std::sync::atomic::Ordering::Relaxed){return Err("已取消下載".into())}
 let next=tokio::select!{chunk=data.next()=>chunk,_=tokio::time::sleep(Duration::from_millis(200))=>continue};let Some(chunk)=next else{break};let chunk=chunk.map_err(|_|"下載中斷")?;done+=chunk.len() as u64;if done>f.size{return Err("內容超出檔案大小".into())}hash.update(&chunk);out.write_all(&chunk).await.map_err(|_|"無法寫入下載")?;if last.elapsed()>Duration::from_millis(250){let speed=done as f64/start.elapsed().as_secs_f64().max(0.001);let _=app.emit("download-progress",serde_json::json!({"done":done,"total":f.size,"speed":speed,"seconds":(f.size-done)as f64/speed.max(1.)}));last=Instant::now();}}
 if done!=f.size||expected.as_ref().is_some_and(|h|h!=&format!("{:x}",hash.finalize())){return Err("完整性檢查失敗，已移除未完成檔案".into())}out.flush().await.map_err(|_|"無法完成下載")?;drop(out);tokio::fs::rename(&partial.0,&path).await.map_err(|_|"無法完成下載")?;Ok(path.to_string_lossy().to_string())}.await;
    c.downloads.lock().unwrap().remove(&file_id);
    result
}
#[tauri::command]
pub fn cancel_download(state: tauri::State<'_, Phone>) -> Result<(), String> {
    for cancel in current(&state)?.downloads.lock().unwrap().values() {
        cancel.store(true, std::sync::atomic::Ordering::Relaxed)
    }
    Ok(())
}
#[tauri::command]
pub fn show_downloads(state: tauri::State<'_, Phone>) -> Result<(), String> {
    let c = current(&state)?;
    std::process::Command::new("explorer.exe")
        .arg(&c.inbox)
        .spawn()
        .map_err(|_| "無法開啟下載資料夾")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn two_computers_pair_metadata_download_reconnect_revoke() {
        let ip = if_addrs::get_if_addrs()
            .unwrap()
            .into_iter()
            .find_map(|i| match i.ip() {
                IpAddr::V4(ip) if local_ip(ip) => Some(ip),
                _ => None,
            })
            .expect("private LAN interface");
        let root = std::env::temp_dir().join(format!("pd-peer-{}", uuid::Uuid::new_v4()));
        let host = Core::open(&root.join("host"), root.join("host-inbox")).unwrap();
        let pc = Core::open(&root.join("pc"), root.join("pc-inbox")).unwrap();
        serve(host.clone(), ip, false).await.unwrap();
        serve(pc.clone(), ip, true).await.unwrap();
        discover(&pc).unwrap();
        for _ in 0..50 {
            if pc
                .nearby
                .lock()
                .unwrap()
                .values()
                .any(|(v, _)| v.device_id == host.device_id)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        assert!(
            pc.nearby
                .lock()
                .unwrap()
                .values()
                .any(|(v, _)| v.device_id == host.device_id),
            "mDNS should find the other identity"
        );
        let invite = host.new_invite().unwrap();
        let address = host.endpoint.lock().unwrap().clone();
        code_pair(&pc, address, invite.code).await.unwrap();
        let p = pc.joined.lock().unwrap().clone().unwrap();
        assert_eq!(load(&pc.dir).unwrap().unwrap().credential, p.credential);
        p.snapshot().await.unwrap();
        let data = b"on demand from original owner";
        let path = root.join("owner-file.stl");
        std::fs::write(&path, data).unwrap();
        pc.add_file(path, "PC").unwrap();
        for _ in 0..100 {
            if pc.local_snapshot().unwrap().files[0].sha256.is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        heartbeat(&pc, &p).await.unwrap();
        let files = host.snapshot().unwrap().files;
        assert_eq!(files.len(), 1);
        assert!(files[0].available);
        assert_eq!(
            std::fs::read_dir(&host.inbox).unwrap().count(),
            0,
            "metadata must not copy payload"
        );
        let json = serde_json::to_string(&host.snapshot().unwrap()).unwrap();
        assert!(!json.contains("local_path"));
        assert!(!json.contains("owner-file.stl\\"));
        let response = p
            .request(
                reqwest::Method::GET,
                &format!("/v1/files/{}", files[0].file_id),
            )
            .unwrap()
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.bytes().await.unwrap().as_ref(), data);
        let response = p
            .request(
                reqwest::Method::GET,
                &format!("/v1/files/{}", files[0].file_id),
            )
            .unwrap()
            .header("range", "bytes=3-8")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(response.bytes().await.unwrap().as_ref(), &data[3..9]);
        host.db
            .lock()
            .unwrap()
            .execute("UPDATE sources SET seen=0", [])
            .unwrap();
        assert!(!host.snapshot().unwrap().files[0].available);
        heartbeat(&pc, &p).await.unwrap();
        assert!(host.snapshot().unwrap().files[0].available);
        host.set_text("shared".into(), &host.device_id).unwrap();
        assert_eq!(p.snapshot().await.unwrap().text, "shared");
        serve(host.clone(), ip, true).await.unwrap();
        let mut moved = p.clone();
        moved.endpoint = host.endpoint.lock().unwrap().clone();
        assert_eq!(moved.snapshot().await.unwrap().text, "shared");
        host.remove_file(&files[0].file_id).unwrap();
        heartbeat(&pc, &moved).await.unwrap();
        assert!(
            host.snapshot().unwrap().files.is_empty(),
            "tombstone must survive owner heartbeat"
        );
        let qr_pc = Core::open(&root.join("qr-pc"), root.join("qr-inbox")).unwrap();
        qr_pair(&qr_pc, host.new_invite().unwrap().qr)
            .await
            .unwrap();
        assert_eq!(
            qr_pc.joined.lock().unwrap().as_ref().unwrap().room_id,
            host.room_id
        );
        drop(qr_pc);
        if let Some(browser) = pc.browser.lock().unwrap().take() {
            let _ = browser.shutdown();
        }
        host.revoke(&pc.device_id).unwrap();
        assert!(moved.snapshot().await.is_err());
        host.transport.lock().unwrap().take();
        pc.transport.lock().unwrap().take();
        tokio::time::sleep(Duration::from_millis(100)).await;
        drop(host);
        drop(pc);
        let _ = std::fs::remove_dir_all(root);
    }
}
