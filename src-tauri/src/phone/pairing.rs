//! SRP-6a / RFC 5054 group 2048, SHA-256; evidence messages match Bouncy Castle.
//! The SRP identity binds the observed TLS certificate and the requested device.
use super::*;
use num_bigint::BigUint;
use srp::{client::SrpClient, groups::G_2048, server::SrpServer};

pub struct CodeInvite {
    pub code: String,
    pub token: String,
    pub attempts: u8,
    pending: Option<Pending>,
}
struct Pending {
    id: String,
    input: Start,
    m1: Vec<u8>,
    m2: Vec<u8>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Start {
    pub device_id: String,
    pub name: String,
    pub public_key: String,
    pub a: String,
}
#[derive(Serialize, Deserialize)]
pub struct Challenge {
    pub id: String,
    pub salt: String,
    pub b: String,
}
#[derive(Serialize, Deserialize)]
pub struct Finish {
    pub id: String,
    pub proof: String,
}
#[derive(Serialize, Deserialize)]
pub struct Completed {
    pub proof: String,
    pub paired: PairOutput,
}
pub fn identity(host: &str, pin: &str, i: &Start) -> Vec<u8> {
    format!(
        "PocketDrop-1\n{host}\n{pin}\n{}\n{}\n{}",
        i.device_id, i.name, i.public_key
    )
    .into_bytes()
}
fn pad(n: &BigUint) -> Vec<u8> {
    let b = n.to_bytes_be();
    let mut out = vec![0; 256 - b.len()];
    out.extend(b);
    out
}
fn digest(parts: &[&[u8]]) -> Vec<u8> {
    let mut h = Sha256::new();
    for p in parts {
        h.update(p)
    }
    h.finalize().to_vec()
}
fn number(s: &str) -> ApiResult<BigUint> {
    let b = STANDARD
        .decode(s)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid_srp"))?;
    if b.is_empty() || b.len() > 256 {
        return Err((StatusCode::BAD_REQUEST, "invalid_srp"));
    }
    let n = BigUint::from_bytes_be(&b);
    if n == BigUint::from(0u8) || n >= G_2048.n {
        return Err((StatusCode::BAD_REQUEST, "invalid_srp"));
    }
    Ok(n)
}
fn random() -> Vec<u8> {
    let mut b = vec![0; 32];
    rand::rngs::OsRng.fill_bytes(&mut b);
    b
}
fn evidence(a: &BigUint, b: &BigUint, s: &BigUint) -> (Vec<u8>, Vec<u8>) {
    let m1 = digest(&[&pad(a), &pad(b), &pad(s)]);
    let m2 = digest(&[&pad(a), &pad(&BigUint::from_bytes_be(&m1)), &pad(s)]);
    (m1, m2)
}
impl CodeInvite {
    pub fn new(token: String) -> Self {
        use rand::Rng;
        Self {
            code: format!("{:08}", rand::rngs::OsRng.gen_range(0..100_000_000u32)),
            token,
            attempts: 0,
            pending: None,
        }
    }
}
pub async fn hello(State(c): State<Arc<Core>>) -> Json<serde_json::Value> {
    Json(
        serde_json::json!({"app":"PocketDrop","protocol_version":1,"device_id":c.device_id,"room_id":c.room_id,"public_key":c.public_key}),
    )
}
pub async fn start(
    State(c): State<Arc<Core>>,
    Json(input): Json<Start>,
) -> ApiResult<Json<Challenge>> {
    if uuid::Uuid::parse_str(&input.device_id).is_err()
        || input.name.is_empty()
        || input.name.chars().count() > 60
        || input.name.chars().any(char::is_control)
        || input.public_key.len() > 1024
        || STANDARD
            .decode(&input.public_key)
            .map_or(true, |b| b.len() < 32)
    {
        return Err((StatusCode::BAD_REQUEST, "invalid_device"));
    }
    let mut invite = c.invite.lock().map_err(internal)?;
    let invite = invite
        .as_mut()
        .filter(|i| i.expires > Instant::now())
        .ok_or((StatusCode::UNAUTHORIZED, "invite_expired"))?;
    if invite.code.attempts >= 5 {
        return Err((StatusCode::TOO_MANY_REQUESTS, "new_invite_required"));
    }
    invite.code.attempts += 1;
    let a = number(&input.a)?;
    let salt = random();
    let secret = BigUint::from_bytes_be(&random());
    let client = SrpClient::<Sha256>::new(&G_2048);
    let server = SrpServer::<Sha256>::new(&G_2048);
    let v = BigUint::from_bytes_be(&client.compute_verifier(
        &identity(&c.device_id, &c.fingerprint, &input),
        invite.code.code.as_bytes(),
        &salt,
    ));
    let k = srp::utils::compute_k::<Sha256>(&G_2048);
    let b = server.compute_b_pub(&secret, &k, &v);
    let u = BigUint::from_bytes_be(&digest(&[&pad(&a), &pad(&b)]));
    let s = server.compute_premaster_secret(&a, &v, &u, &secret);
    let (m1, m2) = evidence(&a, &b, &s);
    let id = random_token();
    invite.code.pending = Some(Pending {
        id: id.clone(),
        input,
        m1,
        m2,
    });
    Ok(Json(Challenge {
        id,
        salt: STANDARD.encode(salt),
        b: STANDARD.encode(b.to_bytes_be()),
    }))
}
pub async fn finish(
    State(c): State<Arc<Core>>,
    Json(input): Json<Finish>,
) -> ApiResult<Json<Completed>> {
    // Consume the proof while holding the invitation lock. core.pair rechecks the
    // one-use invitation; replacing it cannot resurrect an old pending exchange.
    let (pending, token) = {
        let mut guard = c.invite.lock().map_err(internal)?;
        let i = guard
            .as_mut()
            .filter(|i| i.expires > Instant::now())
            .ok_or((StatusCode::UNAUTHORIZED, "invite_expired"))?;
        let p = i
            .code
            .pending
            .take()
            .ok_or((StatusCode::UNAUTHORIZED, "invalid_proof"))?;
        let proof = STANDARD
            .decode(input.proof)
            .map_err(|_| (StatusCode::UNAUTHORIZED, "invalid_proof"))?;
        if p.id != input.id
            || !bool::from(subtle::ConstantTimeEq::ct_eq(
                p.m1.as_slice(),
                proof.as_slice(),
            ))
        {
            return Err((StatusCode::UNAUTHORIZED, "invalid_proof"));
        }
        (p, i.code.token.clone())
    };
    let paired = c.pair(PairInput {
        token,
        device_id: pending.input.device_id,
        name: pending.input.name,
        public_key: pending.input.public_key,
    })?;
    Ok(Json(Completed {
        proof: STANDARD.encode(pending.m2),
        paired,
    }))
}
pub struct ClientProof {
    pub start: Start,
    secret: BigUint,
}
impl ClientProof {
    pub fn new(device_id: String, name: String, public_key: String) -> Self {
        let secret = BigUint::from_bytes_be(&random());
        let a = G_2048.g.modpow(&secret, &G_2048.n);
        Self {
            start: Start {
                device_id,
                name,
                public_key,
                a: STANDARD.encode(a.to_bytes_be()),
            },
            secret,
        }
    }
    pub fn respond(
        &self,
        host: &str,
        pin: &str,
        code: &str,
        ch: &Challenge,
    ) -> Result<(Finish, Vec<u8>), String> {
        if code.len() != 8 || !code.bytes().all(|b| b.is_ascii_digit()) {
            return Err("請輸入 8 位驗證碼".into());
        }
        let b = number(&ch.b).map_err(|_| "配對資料不正確")?;
        let a = number(&self.start.a).map_err(|_| "配對資料不正確")?;
        let salt = STANDARD.decode(&ch.salt).map_err(|_| "配對資料不正確")?;
        if salt.len() != 32 {
            return Err("配對資料不正確".into());
        }
        let ih = SrpClient::<Sha256>::compute_identity_hash(
            &identity(host, pin, &self.start),
            code.as_bytes(),
        );
        let x = SrpClient::<Sha256>::compute_x(&ih, &salt);
        let k = srp::utils::compute_k::<Sha256>(&G_2048);
        let u = BigUint::from_bytes_be(&digest(&[&pad(&a), &pad(&b)]));
        let s = SrpClient::<Sha256>::new(&G_2048).compute_premaster_secret(
            &b,
            &k,
            &x,
            &self.secret,
            &u,
        );
        let (m1, m2) = evidence(&a, &b, &s);
        Ok((
            Finish {
                id: ch.id.clone(),
                proof: STANDARD.encode(m1),
            },
            m2,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn core() -> (PathBuf, Arc<Core>) {
        let dir = std::env::temp_dir().join(format!("pd-code-{}", uuid::Uuid::new_v4()));
        let c = Core::open(&dir, dir.join("inbox")).unwrap();
        *c.endpoint.lock().unwrap() = "https://192.168.1.2:3000".into();
        (dir, c)
    }
    fn client() -> ClientProof {
        ClientProof::new(
            uuid::Uuid::new_v4().to_string(),
            "PC 測試".into(),
            STANDARD.encode([7; 64]),
        )
    }
    #[tokio::test]
    async fn proof_round_trip_one_use_and_certificate_binding() {
        let (dir, c) = core();
        let invitation = c.new_invite().unwrap();
        let client = client();
        let ch = start(State(c.clone()), Json(client.start.clone()))
            .await
            .unwrap()
            .0;
        let (proof, m2) = client
            .respond(&c.device_id, &c.fingerprint, &invitation.code, &ch)
            .unwrap();
        let result = finish(
            State(c.clone()),
            Json(Finish {
                id: proof.id.clone(),
                proof: proof.proof.clone(),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(STANDARD.decode(result.proof).unwrap(), m2);
        assert_eq!(result.paired.room_id, c.room_id);
        assert!(finish(State(c.clone()), Json(proof)).await.is_err());
        let invitation = c.new_invite().unwrap();
        let client = ClientProof::new(
            uuid::Uuid::new_v4().to_string(),
            "PC".into(),
            STANDARD.encode([2; 64]),
        );
        let ch = start(State(c.clone()), Json(client.start.clone()))
            .await
            .unwrap()
            .0;
        let (wrong, _) = client
            .respond(&c.device_id, &"0".repeat(64), &invitation.code, &ch)
            .unwrap();
        assert!(finish(State(c.clone()), Json(wrong)).await.is_err());
        drop(c);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[tokio::test]
    async fn rate_limit_expiry_and_invalid_group_elements() {
        let (dir, c) = core();
        c.new_invite().unwrap();
        for _ in 0..5 {
            let mut input = client().start;
            input.a = STANDARD.encode([0]);
            assert!(start(State(c.clone()), Json(input)).await.is_err());
        }
        assert_eq!(
            start(State(c.clone()), Json(client().start))
                .await
                .err()
                .unwrap()
                .0,
            StatusCode::TOO_MANY_REQUESTS
        );
        c.new_invite().unwrap();
        c.invite.lock().unwrap().as_mut().unwrap().expires = Instant::now();
        assert!(start(State(c.clone()), Json(client().start)).await.is_err());
        drop(c);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
