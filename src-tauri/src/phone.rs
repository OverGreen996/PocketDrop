//! PC-hosted LAN Room.
pub mod pairing;
pub mod peer;
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Path, Query, Request, State, WebSocketUpgrade},
    http::{header, HeaderMap, StatusCode},
    response::Response,
    routing::{get, post, put},
    Json, Router,
};
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine,
};
use futures_util::StreamExt;
use mdns_sd::{IfKind, ServiceDaemon, ServiceInfo};
use rand::RngCore;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    net::{IpAddr, Ipv4Addr, TcpListener},
    path::{Path as FsPath, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{Emitter, Manager};
use tokio::{
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    sync::{watch, Semaphore},
};

pub const MAX_FILE: u64 = 16 * 1024 * 1024 * 1024;
const SERVICE: &str = "_pocketdrop._tcp.local.";
type ApiResult<T> = Result<T, (StatusCode, &'static str)>;
fn internal<E>(_: E) -> (StatusCode, &'static str) {
    (StatusCode::INTERNAL_SERVER_ERROR, "operation_failed")
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn random_token() -> String {
    let mut bytes = [0; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn local_ip(ip: Ipv4Addr) -> bool {
    ip.is_private() || ip.is_link_local()
}
pub fn clean_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    !name.is_empty()
        && name.chars().count() <= 180
        && name != "."
        && name != ".."
        && !name.ends_with([' ', '.'])
        && !name
            .chars()
            .any(|c| c.is_control() || "\\/:*?\"<>|".contains(c))
        && ![
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ]
        .contains(&stem.as_str())
}
pub fn byte_range(header: Option<&str>, size: u64) -> Result<(u64, u64, bool), ()> {
    let Some(value) = header else {
        return Ok((0, size, false));
    };
    if size == 0 {
        return Err(());
    }
    let value = value.strip_prefix("bytes=").ok_or(())?;
    if value.contains(',') {
        return Err(());
    }
    let (start, end) = value.split_once('-').ok_or(())?;
    if start.is_empty() {
        let suffix: u64 = end.parse().map_err(|_| ())?;
        if suffix == 0 {
            return Err(());
        }
        let count = suffix.min(size);
        return Ok((size - count, count, true));
    }
    let start: u64 = start.parse().map_err(|_| ())?;
    let end = if end.is_empty() {
        size - 1
    } else {
        end.parse::<u64>().map_err(|_| ())?.min(size - 1)
    };
    if start >= size || end < start {
        return Err(());
    }
    Ok((start, end - start + 1, true))
}

#[cfg(windows)]
fn protect(data: &[u8], decrypt: bool) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{
            CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        },
    };
    let input = CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    unsafe {
        let ok = if decrypt {
            CryptUnprotectData(
                &input,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptProtectData(
                &input,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        if ok == 0 {
            return Err("Windows 金鑰保護失敗".into());
        }
        let result = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        LocalFree(output.pbData as _);
        Ok(result)
    }
}
#[cfg(not(windows))]
fn protect(_: &[u8], _: bool) -> Result<Vec<u8>, String> {
    Err("Windows only".into())
}

struct Invite {
    token_hash: String,
    expires: Instant,
    code: pairing::CodeInvite,
}
pub struct Core {
    db: Mutex<Connection>,
    pub room_id: String,
    pub device_id: String,
    pub cert_pem: Vec<u8>,
    key_pem: Vec<u8>,
    fingerprint: String,
    public_key: String,
    inbox: PathBuf,
    invite: Mutex<Option<Invite>>,
    endpoint: Mutex<String>,
    revision: watch::Sender<u64>,
    uploads: Arc<Semaphore>,
    transport: Mutex<Option<Transport>>,
    hash_gate: Mutex<()>,
    dir: PathBuf,
    joined: Mutex<Option<peer::Profile>>,
    cached: Mutex<Option<Snapshot>>,
    nearby: Mutex<std::collections::HashMap<String, (peer::NearbyRoom, Instant)>>,
    browser: Mutex<Option<ServiceDaemon>>,
    callback: Mutex<Option<(String, String)>>,
    downloads: Mutex<std::collections::HashMap<String, Arc<std::sync::atomic::AtomicBool>>>,
}
struct Transport {
    daemon: ServiceDaemon,
    fullname: String,
    handle: axum_server::Handle,
}
impl Drop for Transport {
    fn drop(&mut self) {
        self.handle.shutdown();
        let _ = self.daemon.unregister(&self.fullname);
        let _ = self.daemon.shutdown();
    }
}
pub struct Phone(pub Mutex<Option<Arc<Core>>>);

#[derive(Clone, Serialize, Deserialize)]
pub struct FileView {
    pub file_id: String,
    pub name: String,
    pub size: u64,
    pub origin: String,
    pub available: bool,
    pub sha256: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct DeviceView {
    device_id: String,
    name: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub room_id: String,
    pub device_id: String,
    pub room_name: String,
    pub text: String,
    pub revision: u64,
    pub files: Vec<FileView>,
    pub devices: Vec<DeviceView>,
    pub endpoint: String,
}
#[derive(Serialize)]
pub struct InviteView {
    pub qr: String,
    pub expires_seconds: u32,
    pub code: String,
}
#[derive(Serialize, Deserialize)]
pub struct PairInput {
    token: String,
    device_id: String,
    name: String,
    public_key: String,
}
#[derive(Serialize, Deserialize)]
pub struct PairOutput {
    pub credential: String,
    pub room_id: String,
    pub device_id: String,
    pub room_name: String,
}
#[derive(Deserialize)]
struct TextInput {
    content: String,
}
#[derive(Deserialize)]
struct UploadQuery {
    name: String,
}

impl Core {
    pub fn open(dir: &FsPath, inbox: PathBuf) -> Result<Arc<Self>, String> {
        std::fs::create_dir_all(dir).map_err(|_| "無法建立資料目錄")?;
        std::fs::create_dir_all(&inbox).map_err(|_| "無法建立接收資料夾")?;
        let db = Connection::open(dir.join("phone-trial.sqlite")).map_err(|e| e.to_string())?;
        db.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE IF NOT EXISTS meta(k TEXT PRIMARY KEY,v TEXT NOT NULL); CREATE TABLE IF NOT EXISTS members(id TEXT PRIMARY KEY,name TEXT NOT NULL,token_hash TEXT NOT NULL,pubkey TEXT NOT NULL,revoked INTEGER NOT NULL DEFAULT 0); CREATE TABLE IF NOT EXISTS history(version INTEGER PRIMARY KEY,content TEXT NOT NULL,source TEXT NOT NULL,at INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS files(id TEXT PRIMARY KEY,name TEXT NOT NULL,size INTEGER NOT NULL,origin TEXT NOT NULL,deleted INTEGER NOT NULL DEFAULT 0,sha256 TEXT); CREATE TABLE IF NOT EXISTS bindings(file_id TEXT PRIMARY KEY,local_path TEXT NOT NULL,modified INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS sources(owner TEXT PRIMARY KEY,data BLOB NOT NULL,seen INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS remote_ids(owner TEXT,source TEXT,id TEXT UNIQUE,deleted INTEGER NOT NULL DEFAULT 0,PRIMARY KEY(owner,source)); INSERT OR IGNORE INTO meta VALUES('revision','0'); INSERT OR IGNORE INTO meta VALUES('text','');").map_err(|e|e.to_string())?;
        for key in ["room_id", "device_id"] {
            db.execute(
                "INSERT OR IGNORE INTO meta VALUES(?1,?2)",
                params![key, uuid::Uuid::new_v4().to_string()],
            )
            .map_err(|e| e.to_string())?;
        }
        let room_id = db
            .query_row("SELECT v FROM meta WHERE k='room_id'", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        let device_id = db
            .query_row("SELECT v FROM meta WHERE k='device_id'", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        let cert_path = dir.join("identity-cert.pem");
        let key_path = dir.join("identity-key.dpapi");
        let pub_path = dir.join("identity-public.txt");
        if !cert_path.exists() || !key_path.exists() || !pub_path.exists() {
            // Refuse to silently rotate an established identity if one key file is missing.
            if cert_path.exists() || key_path.exists() || pub_path.exists() {
                return Err("身分檔案不完整，請恢復備份；不會自動更换金鑰".into());
            }
            let key = rcgen::generate_simple_self_signed(vec!["pocketdrop.local".into()])
                .map_err(|e| e.to_string())?;
            std::fs::write(
                &key_path,
                protect(key.key_pair.serialize_pem().as_bytes(), false)?,
            )
            .map_err(|e| e.to_string())?;
            std::fs::write(&pub_path, STANDARD.encode(key.key_pair.public_key_der()))
                .map_err(|e| e.to_string())?;
            std::fs::write(&cert_path, key.cert.pem()).map_err(|e| e.to_string())?;
        }
        let cert_pem = std::fs::read(cert_path).map_err(|e| e.to_string())?;
        let raw = String::from_utf8_lossy(&cert_pem)
            .lines()
            .filter(|line| !line.starts_with('-'))
            .collect::<String>();
        let fingerprint = hash(&STANDARD.decode(raw).map_err(|e| e.to_string())?);
        let key_pem = protect(&std::fs::read(key_path).map_err(|e| e.to_string())?, true)?;
        let public_key = std::fs::read_to_string(pub_path).map_err(|e| e.to_string())?;
        let (revision, _) = watch::channel(0);
        Ok(Arc::new(Self {
            db: Mutex::new(db),
            room_id,
            device_id,
            cert_pem,
            key_pem,
            fingerprint,
            public_key,
            inbox,
            invite: Mutex::new(None),
            endpoint: Mutex::new(String::new()),
            revision,
            uploads: Arc::new(Semaphore::new(2)),
            transport: Mutex::new(None),
            hash_gate: Mutex::new(()),
            dir: dir.to_path_buf(),
            joined: Mutex::new(peer::load(dir)?),
            cached: Mutex::new(None),
            nearby: Mutex::new(Default::default()),
            browser: Mutex::new(None),
            callback: Mutex::new(None),
            downloads: Mutex::new(Default::default()),
        }))
    }
    fn changed(&self) {
        self.revision.send_modify(|v| *v += 1);
    }
    pub fn snapshot(&self) -> Result<Snapshot, String> {
        let mut s = self.local_snapshot()?;
        s.files.extend(peer::remote_files(self)?);
        Ok(s)
    }
    pub fn local_snapshot(&self) -> Result<Snapshot, String> {
        let db = self.db.lock().map_err(|_| "資料庫忙碌")?;
        let text = db
            .query_row("SELECT v FROM meta WHERE k='text'", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        let revision = db
            .query_row(
                "SELECT CAST(v AS INTEGER) FROM meta WHERE k='revision'",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let mut stmt=db.prepare("SELECT f.id,f.name,f.size,f.origin,f.sha256,b.local_path,b.modified FROM files f JOIN bindings b ON f.id=b.file_id WHERE deleted=0 ORDER BY f.rowid DESC LIMIT 200").map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, u64>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, u64>(6)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut files = vec![];
        for row in rows {
            let (id, name, size, origin, sha256, path, modified) =
                row.map_err(|e| e.to_string())?;
            let available =
                file_stamp(FsPath::new(&path)).is_some_and(|(s, m)| s == size && m == modified);
            files.push(FileView {
                file_id: id,
                name,
                size,
                origin,
                available,
                sha256,
            });
        }
        let mut stmt = db
            .prepare("SELECT id,name FROM members WHERE revoked=0 AND pubkey != 'internal'")
            .map_err(|e| e.to_string())?;
        let devices = stmt
            .query_map([], |r| {
                Ok(DeviceView {
                    device_id: r.get(0)?,
                    name: r.get(1)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(Snapshot {
            room_id: self.room_id.clone(),
            device_id: self.device_id.clone(),
            room_name: "PocketDrop Room".into(),
            text,
            revision,
            files,
            devices,
            endpoint: self.endpoint.lock().unwrap().clone(),
        })
    }
    pub fn new_invite(&self) -> Result<InviteView, String> {
        if self.joined.lock().unwrap().is_some() {
            return Err("請在建立 Room 的電腦邀請裝置".into());
        }
        let endpoint = self.endpoint.lock().unwrap().clone();
        if endpoint.is_empty() {
            return Err("請先連接 Wi-Fi／乙太網路".into());
        }
        let token = random_token();
        *self.invite.lock().unwrap() = Some(Invite {
            token_hash: hash(token.as_bytes()),
            expires: Instant::now() + Duration::from_secs(300),
            code: pairing::CodeInvite::new(token.clone()),
        });
        let qr=serde_json::json!({"app":"PocketDrop","protocol_version":1,"mode":"phone-trial","room_id":self.room_id,"device_id":self.device_id,"public_key":self.public_key,"certificate_sha256":self.fingerprint,"token":token,"endpoint":endpoint}).to_string();
        Ok(InviteView {
            qr,
            expires_seconds: 300,
            code: self
                .invite
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .code
                .code
                .clone(),
        })
    }
    pub fn pair(&self, input: PairInput) -> ApiResult<PairOutput> {
        if uuid::Uuid::parse_str(&input.device_id).is_err()
            || input.name.is_empty()
            || input.name.chars().count() > 60
            || input.name.chars().any(char::is_control)
            || input.public_key.len() > 1024
            || STANDARD
                .decode(&input.public_key)
                .map_or(true, |v| v.len() < 32)
        {
            return Err((StatusCode::BAD_REQUEST, "invalid_device"));
        }
        let mut invite = self.invite.lock().map_err(internal)?;
        let valid = invite.as_ref().is_some_and(|i| {
            i.expires > Instant::now()
                && subtle::ConstantTimeEq::ct_eq(
                    i.token_hash.as_bytes(),
                    hash(input.token.as_bytes()).as_bytes(),
                )
                .into()
        });
        if !valid {
            return Err((StatusCode::UNAUTHORIZED, "invite_expired_or_used"));
        }
        let credential = random_token();
        self.db.lock().map_err(internal)?.execute("INSERT INTO members(id,name,token_hash,pubkey,revoked) VALUES(?1,?2,?3,?4,0) ON CONFLICT(id) DO UPDATE SET name=excluded.name,token_hash=excluded.token_hash,pubkey=excluded.pubkey,revoked=0",params![input.device_id,input.name,hash(credential.as_bytes()),input.public_key]).map_err(internal)?;
        *invite = None;
        drop(invite);
        self.changed();
        Ok(PairOutput {
            credential,
            room_id: self.room_id.clone(),
            device_id: self.device_id.clone(),
            room_name: "PocketDrop Room".into(),
        })
    }
    fn authorize(&self, headers: &HeaderMap) -> ApiResult<(String, String)> {
        let token = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .filter(|s| s.len() == 43)
            .ok_or((StatusCode::UNAUTHORIZED, "not_paired"))?;
        let digest = hash(token.as_bytes());
        let id = self
            .db
            .lock()
            .map_err(internal)?
            .query_row(
                "SELECT id FROM members WHERE token_hash=?1 AND revoked=0",
                [&digest],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .map_err(internal)?
            .ok_or((StatusCode::UNAUTHORIZED, "device_revoked"))?;
        if headers
            .get("x-pocketdrop-room")
            .and_then(|v| v.to_str().ok())
            != Some(self.room_id.as_str())
        {
            return Err((StatusCode::FORBIDDEN, "wrong_room"));
        }
        if self
            .joined
            .lock()
            .map_err(internal)?
            .as_ref()
            .is_some_and(|p| p.device_id != id)
        {
            return Err((StatusCode::FORBIDDEN, "room_inactive"));
        }
        Ok((id, digest))
    }
    fn valid_digest(&self, digest: &str) -> bool {
        let id = self.db.lock().ok().and_then(|db| {
            db.query_row(
                "SELECT id FROM members WHERE token_hash=?1 AND revoked=0",
                [digest],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .ok()
            .flatten()
        });
        id.is_some_and(|id| {
            self.joined
                .lock()
                .ok()
                .is_some_and(|p| p.as_ref().is_none_or(|p| p.device_id == id))
        })
    }
    pub fn set_text(&self, content: String, source: &str) -> Result<(), String> {
        if content.len() > 32768 {
            return Err("文字最多 32 KB".into());
        }
        let mut db = self.db.lock().map_err(|_| "資料庫忙碌")?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE meta SET v=CAST(v AS INTEGER)+1 WHERE k='revision'",
            [],
        )
        .map_err(|e| e.to_string())?;
        tx.execute("UPDATE meta SET v=?1 WHERE k='text'", [&content])
            .map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO history VALUES((SELECT CAST(v AS INTEGER) FROM meta WHERE k='revision'),?1,?2,?3)",params![content,source,now()]).map_err(|e|e.to_string())?;
        tx.execute("DELETE FROM history WHERE version NOT IN (SELECT version FROM history ORDER BY version DESC LIMIT 20)",[]).map_err(|e|e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        drop(db);
        self.changed();
        Ok(())
    }
    pub fn revoke(&self, id: &str) -> Result<(), String> {
        self.db
            .lock()
            .map_err(|_| "資料庫忙碌")?
            .execute("UPDATE members SET revoked=1 WHERE id=?1", [id])
            .map_err(|e| e.to_string())?;
        self.changed();
        Ok(())
    }
    pub fn remove_file(&self, id: &str) -> Result<(), String> {
        self.db
            .lock()
            .map_err(|_| "資料庫忙碌")?
            .execute("UPDATE files SET deleted=1 WHERE id=?1", [id])
            .map_err(|e| e.to_string())?;
        self.db
            .lock()
            .map_err(|_| "資料庫忙碌")?
            .execute("UPDATE remote_ids SET deleted=1 WHERE id=?1", [id])
            .map_err(|e| e.to_string())?;
        self.changed();
        Ok(())
    }
    pub fn add_file(self: &Arc<Self>, path: PathBuf, origin: &str) -> Result<(), String> {
        let name = path
            .file_name()
            .ok_or("無效檔名")?
            .to_string_lossy()
            .to_string();
        if !clean_name(&name) || path.to_string_lossy().starts_with(r"\\") {
            return Err("不支援此檔名或網路路徑".into());
        }
        let (size, modified) =
            file_stamp(&path).ok_or("目前僅支援本機檔案，請先將資料夾壓縮成 ZIP")?;
        if size > MAX_FILE {
            return Err("每個檔案最多 16 GiB".into());
        }
        let id = uuid::Uuid::new_v4().to_string();
        {
            let mut db = self.db.lock().map_err(|_| "資料庫忙碌")?;
            let count: u32 = db
                .query_row("SELECT count(*) FROM files WHERE deleted=0", [], |r| {
                    r.get(0)
                })
                .map_err(|e| e.to_string())?;
            if count >= 200 {
                return Err("最多保留 200 個檔案紀錄".into());
            }
            let tx = db.transaction().map_err(|e| e.to_string())?;
            tx.execute(
                "INSERT INTO files(id,name,size,origin) VALUES(?1,?2,?3,?4)",
                params![id, name, size, origin],
            )
            .map_err(|e| e.to_string())?;
            tx.execute(
                "INSERT INTO bindings VALUES(?1,?2,?3)",
                params![id, path.to_string_lossy(), modified],
            )
            .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
        }
        self.changed();
        let core = self.clone();
        std::thread::spawn(move || {
            let Ok(_gate) = core.hash_gate.lock() else {
                return;
            };
            use std::io::Read;
            let Ok(mut f) = std::fs::File::open(&path) else {
                return;
            };
            let mut h = Sha256::new();
            let mut buf = [0; 65536];
            loop {
                match f.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => h.update(&buf[..n]),
                    Err(_) => return,
                }
            }
            if file_stamp(&path) != Some((size, modified)) {
                return;
            }
            if let Ok(db) = core.db.lock() {
                let _ = db.execute(
                    "UPDATE files SET sha256=?1 WHERE id=?2",
                    params![format!("{:x}", h.finalize()), id],
                );
            }
            core.changed();
        });
        Ok(())
    }
}
fn file_stamp(path: &FsPath) -> Option<(u64, u64)> {
    let m = std::fs::symlink_metadata(path).ok()?;
    if !m.is_file() || m.file_type().is_symlink() {
        return None;
    }
    Some((
        m.len(),
        m.modified()
            .ok()?
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_nanos()
            .min(u64::MAX as u128) as u64,
    ))
}

pub async fn serve(core: Arc<Core>, ip: Ipv4Addr, smoke: bool) -> Result<(), String> {
    if !(local_ip(ip) || (smoke && ip.is_loopback())) {
        return Err("僅允許 LAN 位址".into());
    }
    if !smoke
        && !if_addrs::get_if_addrs()
            .map_err(|e| e.to_string())?
            .iter()
            .any(|i| i.ip() == IpAddr::V4(ip))
    {
        return Err("此網路已離線".into());
    }
    core.transport.lock().unwrap().take();
    core.endpoint.lock().unwrap().clear();
    core.invite.lock().unwrap().take();
    let _ = rustls::crypto::ring::default_provider().install_default();
    let listener = TcpListener::bind((ip, 0)).map_err(|e| e.to_string())?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let tls = axum_server::tls_rustls::RustlsConfig::from_pem(
        core.cert_pem.clone(),
        core.key_pem.clone(),
    )
    .await
    .map_err(|e| e.to_string())?;
    let daemon = ServiceDaemon::new().map_err(|e| e.to_string())?;
    daemon
        .disable_interface(IfKind::All)
        .map_err(|e| e.to_string())?;
    let props = [
        ("app", "PocketDrop"),
        ("pv", "1"),
        ("id", core.device_id.as_str()),
        ("cap", "room-code-v1"),
    ];
    let service = ServiceInfo::new(
        SERVICE,
        &format!("PocketDrop-{}", &core.device_id[..8]),
        &format!("pd-{}.local.", core.device_id),
        ip.to_string().as_str(),
        port,
        &props[..],
    )
    .map_err(|e| e.to_string())?;
    let fullname = service.get_fullname().to_string();
    if !smoke {
        daemon
            .enable_interface(IfKind::Addr(IpAddr::V4(ip)))
            .map_err(|e| e.to_string())?;
        daemon.register(service).map_err(|e| e.to_string())?;
    }
    let router = Router::new()
        .route("/v1/hello", get(pairing::hello))
        .route("/v1/code/start", post(pairing::start))
        .route("/v1/code/finish", post(pairing::finish))
        .route(
            "/v1/source",
            put(peer::source).layer(DefaultBodyLimit::max(262144)),
        )
        .route("/v1/pair", post(pair))
        .route("/v1/state", get(state))
        .route("/v1/text", put(text_update))
        .route("/v1/events", get(events))
        .route("/v1/files/{id}", get(download))
        .route("/v1/files", put(upload).layer(DefaultBodyLimit::disable()))
        .layer(DefaultBodyLimit::max(65536))
        .with_state(core.clone());
    let handle = axum_server::Handle::new();
    let server_handle = handle.clone();
    *core.endpoint.lock().unwrap() = format!("https://{ip}:{port}");
    *core.transport.lock().unwrap() = Some(Transport {
        daemon,
        fullname,
        handle,
    });
    let server_endpoint = core.endpoint.lock().unwrap().clone();
    tokio::spawn(async move {
        if axum_server::from_tcp_rustls(listener, tls)
            .handle(server_handle)
            .serve(router.into_make_service())
            .await
            .is_err()
        {
            let mut endpoint = core.endpoint.lock().unwrap();
            if *endpoint == server_endpoint {
                endpoint.clear();
            }
            drop(endpoint);
            core.changed();
            eprintln!("{{\"event\":\"phone_listener_failed\"}}");
        }
    });
    Ok(())
}
async fn pair(
    State(c): State<Arc<Core>>,
    Json(input): Json<PairInput>,
) -> ApiResult<Json<PairOutput>> {
    Ok(Json(c.pair(input)?))
}
async fn state(State(c): State<Arc<Core>>, headers: HeaderMap) -> ApiResult<Json<Snapshot>> {
    c.authorize(&headers)?;
    Ok(Json(c.snapshot().map_err(internal)?))
}
async fn text_update(
    State(c): State<Arc<Core>>,
    headers: HeaderMap,
    Json(input): Json<TextInput>,
) -> ApiResult<StatusCode> {
    let (id, _) = c.authorize(&headers)?;
    if input.content.len() > 32768 {
        return Err((StatusCode::PAYLOAD_TOO_LARGE, "text_too_large"));
    }
    c.set_text(input.content, &id).map_err(internal)?;
    Ok(StatusCode::NO_CONTENT)
}
async fn events(
    State(c): State<Arc<Core>>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> ApiResult<Response> {
    let (_, digest) = c.authorize(&headers)?;
    Ok(ws.max_message_size(1024).on_upgrade(move |mut socket|async move {
        let mut changes=c.revision.subscribe();let mut heartbeat=tokio::time::interval(Duration::from_secs(2));
        loop {
            tokio::select! { _=changes.changed()=>{}, _=heartbeat.tick()=>{}, msg=socket.recv()=>{match msg {Some(Ok(axum::extract::ws::Message::Close(_)))|None|Some(Err(_))=>break,_=>continue}} }
            if !c.valid_digest(&digest){let _=socket.send(axum::extract::ws::Message::Close(None)).await;break;}
            if socket.send(axum::extract::ws::Message::Text("{\"event\":\"refresh\"}".into())).await.is_err(){break;}
        }
    }))
}
async fn download(
    State(c): State<Arc<Core>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let (_, digest) = c.authorize(&headers)?;
    let remote =
        c.db.lock()
            .map_err(internal)?
            .query_row("SELECT 1 FROM remote_ids WHERE id=?1", [&id], |r| {
                r.get::<_, u8>(0)
            })
            .optional()
            .map_err(internal)?
            .is_some();
    if remote {
        return peer::remote_download(c, id, headers, digest).await;
    }
    let row=c.db.lock().map_err(internal)?.query_row("SELECT f.size,b.local_path,b.modified,f.sha256 FROM files f JOIN bindings b ON f.id=b.file_id WHERE f.id=?1 AND f.deleted=0",[&id],|r|Ok((r.get::<_,u64>(0)?,r.get::<_,String>(1)?,r.get::<_,u64>(2)?,r.get::<_,Option<String>>(3)?))).optional().map_err(internal)?.ok_or((StatusCode::NOT_FOUND,"file_removed"))?;
    if file_stamp(FsPath::new(&row.1)) != Some((row.0, row.2)) {
        return Err((StatusCode::GONE, "source_unavailable"));
    }
    let (start, count, partial) = byte_range(
        headers.get(header::RANGE).and_then(|v| v.to_str().ok()),
        row.0,
    )
    .map_err(|_| (StatusCode::RANGE_NOT_SATISFIABLE, "invalid_range"))?;
    let mut file = tokio::fs::File::open(&row.1)
        .await
        .map_err(|_| (StatusCode::GONE, "source_unavailable"))?;
    file.seek(std::io::SeekFrom::Start(start))
        .await
        .map_err(internal)?;
    let stream = futures_util::stream::try_unfold(
        (file, count, c, digest),
        |(mut file, left, core, digest)| async move {
            if left == 0 {
                return Ok::<_, std::io::Error>(None);
            }
            if !core.valid_digest(&digest) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "revoked",
                ));
            }
            let mut bytes = vec![0; left.min(65536) as usize];
            let n = file.read(&mut bytes).await?;
            if n == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "changed",
                ));
            }
            bytes.truncate(n);
            Ok(Some((bytes, (file, left - n as u64, core, digest))))
        },
    );
    let mut response = Response::builder()
        .status(if partial {
            StatusCode::PARTIAL_CONTENT
        } else {
            StatusCode::OK
        })
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CONTENT_DISPOSITION, "attachment")
        .header(header::CONTENT_LENGTH, count)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CACHE_CONTROL, "no-store")
        .header("x-content-type-options", "nosniff");
    if partial {
        response = response.header(
            header::CONTENT_RANGE,
            format!("bytes {}-{}/{}", start, start + count - 1, row.0),
        );
    }
    if let Some(h) = row.3 {
        response = response.header("x-content-sha256", h);
    }
    response.body(Body::from_stream(stream)).map_err(internal)
}
struct PartialFile(PathBuf);
impl Drop for PartialFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
async fn upload(
    State(c): State<Arc<Core>>,
    Query(q): Query<UploadQuery>,
    request: Request,
) -> ApiResult<StatusCode> {
    let (id, digest) = c.authorize(request.headers())?;
    if !clean_name(&q.name) {
        return Err((StatusCode::BAD_REQUEST, "invalid_filename"));
    }
    let expected = request
        .headers()
        .get("x-file-size")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|n| *n <= MAX_FILE)
        .ok_or((StatusCode::BAD_REQUEST, "invalid_size"))?;
    let _permit = c
        .uploads
        .clone()
        .try_acquire_owned()
        .map_err(|_| (StatusCode::TOO_MANY_REQUESTS, "busy"))?;
    let sub = c.inbox.join(uuid::Uuid::new_v4().to_string());
    tokio::fs::create_dir(&sub).await.map_err(internal)?;
    let partial = PartialFile(sub.join(format!(".{}.part", uuid::Uuid::new_v4())));
    let final_path = sub.join(&q.name);
    let mut out = tokio::fs::File::create(&partial.0)
        .await
        .map_err(internal)?;
    let mut received = 0u64;
    let mut data = request.into_body().into_data_stream();
    while let Some(chunk) = tokio::time::timeout(Duration::from_secs(30), data.next())
        .await
        .map_err(|_| (StatusCode::REQUEST_TIMEOUT, "upload_timeout"))?
    {
        let chunk = chunk.map_err(|_| (StatusCode::BAD_REQUEST, "upload_cancelled"))?;
        if !c.valid_digest(&digest) {
            return Err((StatusCode::UNAUTHORIZED, "device_revoked"));
        }
        received = received
            .checked_add(chunk.len() as u64)
            .ok_or((StatusCode::PAYLOAD_TOO_LARGE, "invalid_size"))?;
        if received > expected {
            return Err((StatusCode::PAYLOAD_TOO_LARGE, "size_mismatch"));
        }
        out.write_all(&chunk).await.map_err(internal)?;
    }
    if received != expected {
        return Err((StatusCode::BAD_REQUEST, "size_mismatch"));
    }
    out.flush().await.map_err(internal)?;
    drop(out);
    tokio::fs::rename(&partial.0, &final_path)
        .await
        .map_err(internal)?;
    if let Err(_) = c.add_file(final_path.clone(), &format!("手機 {}", &id[..8])) {
        let _ = tokio::fs::remove_file(final_path).await;
        return Err((StatusCode::CONFLICT, "file_limit"));
    }
    Ok(StatusCode::CREATED)
}

pub fn current(state: &Phone) -> Result<Arc<Core>, String> {
    state
        .0
        .lock()
        .map_err(|_| "狀態忙碌")?
        .clone()
        .ok_or("Room 尚未啟動".into())
}
#[tauri::command]
pub async fn phone_start(
    app: tauri::AppHandle,
    state: tauri::State<'_, Phone>,
    address: String,
) -> Result<Snapshot, String> {
    let existing = state.0.lock().map_err(|_| "狀態忙碌")?.clone();
    let c = if let Some(c) = existing {
        c
    } else {
        let dir = app
            .path()
            .app_data_dir()
            .map_err(|e| e.to_string())?
            .join("phone-trial");
        let inbox = app
            .path()
            .download_dir()
            .map_err(|e| e.to_string())?
            .join("PocketDrop");
        let c = Core::open(&dir, inbox)?;
        *state.0.lock().unwrap() = Some(c.clone());
        let handle = app.clone();
        let core = c.clone();
        tauri::async_runtime::spawn(async move {
            let mut changes = core.revision.subscribe();
            while changes.changed().await.is_ok() {
                let _ = handle.emit("phone-changed", ());
            }
        });
        c
    };
    let ip = address.parse().map_err(|_| "請選擇可用網路")?;
    serve(c.clone(), ip, false).await?;
    if c.browser.lock().unwrap().is_none() {
        peer::discover(&c)?;
        peer::background(c.clone(), app.clone());
    }
    let profile = c.joined.lock().unwrap().clone();
    if let Some(p) = profile {
        if let Ok(s) = p.snapshot().await {
            *c.cached.lock().unwrap() = Some(s.clone());
            return Ok(s);
        }
    }
    c.snapshot()
}
#[tauri::command]
pub async fn phone_snapshot(state: tauri::State<'_, Phone>) -> Result<Snapshot, String> {
    let c = current(&state)?;
    if c.joined.lock().unwrap().is_some() {
        return c
            .cached
            .lock()
            .unwrap()
            .clone()
            .ok_or("正在重新尋找 Room，配對已保存".into());
    }
    c.snapshot()
}
#[tauri::command]
pub fn phone_invite(state: tauri::State<'_, Phone>) -> Result<InviteView, String> {
    current(&state)?.new_invite()
}
#[tauri::command]
pub async fn phone_text(state: tauri::State<'_, Phone>, content: String) -> Result<(), String> {
    let c = current(&state)?;
    let p = c.joined.lock().unwrap().clone();
    if let Some(p) = p {
        if content.len() > 32768 {
            return Err("文字最多 32 KB".into());
        }
        peer::checked(
            p.request(reqwest::Method::PUT, "/v1/text")?
                .timeout(Duration::from_secs(8))
                .json(&serde_json::json!({"content":content}))
                .send()
                .await
                .map_err(|_| "連線中斷，草稿已保留")?,
        )
        .await?;
        *c.cached.lock().unwrap() = Some(p.snapshot().await?);
        Ok(())
    } else {
        c.set_text(content, &c.device_id)
    }
}
#[tauri::command]
pub fn phone_revoke(state: tauri::State<'_, Phone>, device_id: String) -> Result<(), String> {
    current(&state)?.revoke(&device_id)
}
#[tauri::command]
pub fn phone_remove(state: tauri::State<'_, Phone>, file_id: String) -> Result<(), String> {
    current(&state)?.remove_file(&file_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filenames_reject_paths_and_windows_devices() {
        for name in ["../x", "C:\\secret", "CON.txt", "a/b", "file.", "x\n.exe"] {
            assert!(!clean_name(name));
        }
        assert!(clean_name("模型.stl"));
    }
    #[test]
    fn ranges_are_bounded() {
        assert_eq!(byte_range(Some("bytes=3-6"), 10), Ok((3, 4, true)));
        assert_eq!(byte_range(Some("bytes=-4"), 10), Ok((6, 4, true)));
        for s in ["bytes=10-", "bytes=5-2", "bytes=0-1,4-5", "bytes=-0"] {
            assert!(byte_range(Some(s), 10).is_err())
        }
    }
    #[test]
    fn pair_once_revoke_persist_and_no_paths() {
        let root = std::env::temp_dir().join(format!("pd-{}", uuid::Uuid::new_v4()));
        let core = Core::open(&root, root.join("inbox")).unwrap();
        *core.endpoint.lock().unwrap() = "https://192.168.1.2:1".into();
        let qr: serde_json::Value = serde_json::from_str(&core.new_invite().unwrap().qr).unwrap();
        let input = || PairInput {
            token: qr["token"].as_str().unwrap().into(),
            device_id: "c75be8f1-55f0-4f5f-a157-1b27a41c881d".into(),
            name: "test".into(),
            public_key: STANDARD.encode([0; 64]),
        };
        let result = core.pair(input()).unwrap();
        assert!(core.pair(input()).is_err());
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            format!("Bearer {}", result.credential).parse().unwrap(),
        );
        headers.insert("x-pocketdrop-room", core.room_id.parse().unwrap());
        assert!(core.authorize(&headers).is_ok());
        core.set_text("hello".into(), "test").unwrap();
        core.revoke(&input().device_id).unwrap();
        assert!(core.authorize(&headers).is_err());
        let json = serde_json::to_string(&core.snapshot().unwrap()).unwrap();
        assert!(!json.contains("local_path"));
        drop(core);
        let core = Core::open(&root, root.join("inbox")).unwrap();
        assert_eq!(core.snapshot().unwrap().text, "hello");
        assert!(core.authorize(&headers).is_err());
        drop(core);
        std::fs::remove_dir_all(root).unwrap();
    }
}
