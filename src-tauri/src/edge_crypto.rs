//! `dx150-edge-v1` 分域加密网关客户端。
//!
//! 上游已下线旧明文信封 `POST /api/edge/request`，将所有接口按作用域拆分为：
//! `/api/public/request`、`/api/dynamic/request`、`/api/playback/request`、`/api/user/request`，
//! 并使用 ECDH P-256 + HKDF-SHA256 + AES-256-GCM 会话加密通信（`public` 作用域同时支持
//! `X-HG-Public-JSON: 1` 快速 JSON 通道，失败时自动回退到会话加密）。

use base64::engine::general_purpose::{STANDARD as B64_STD, STANDARD_NO_PAD as B64_STD_NO_PAD, URL_SAFE_NO_PAD as B64_URL};
use base64::Engine;
use once_cell::sync::Lazy;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::agreement::{agree_ephemeral, EphemeralPrivateKey, UnparsedPublicKey, ECDH_P256};
use ring::hkdf::{KeyType, Salt, HKDF_SHA256};
use ring::rand::SystemRandom;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex as AsyncMutex;

const PROTOCOL: &str = "dx150-edge-v1";
const MAX_SEQUENCE: u32 = 16384;
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(12);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scope {
    Public,
    Dynamic,
    Playback,
    User,
}

impl Scope {
    pub fn as_str(self) -> &'static str {
        match self {
            Scope::Public => "public",
            Scope::Dynamic => "dynamic",
            Scope::Playback => "playback",
            Scope::User => "user",
        }
    }

    pub fn endpoint(self) -> &'static str {
        match self {
            Scope::Public => "/api/public/request",
            Scope::Dynamic => "/api/dynamic/request",
            Scope::Playback => "/api/playback/request",
            Scope::User => "/api/user/request",
        }
    }
}

/// 按 `__cloudfrontCompat.encryptedAPIRoutes` 规则将业务请求映射到对应网关作用域。
pub fn route_scope(method: &str, raw_path: &str) -> Scope {
    let m = match method.trim().to_ascii_uppercase().as_str() {
        "HEAD" => "GET".to_string(),
        other => other.to_string(),
    };
    let (pathname, query) = raw_path.split_once('?').unwrap_or((raw_path, ""));
    let segs: Vec<&str> = pathname.trim_start_matches('/').split('/').collect();

    // 1. Playback scope
    match (m.as_str(), segs.as_slice()) {
        ("POST", ["api", "videos", _, "episodes", _, "playback"])
        | ("GET", ["api", "videos", _, "play"])
        | ("POST", ["api", "videos", _, "playback"])
        | ("POST", ["api", "posts", _, "playback"])
        | ("POST", ["api", "media", "playback", "previews"])
        | ("POST", ["api", "media", "prerolls", _, "playback"]) => return Scope::Playback,
        _ => {}
    }

    // 2. User scope
    if matches!(segs.as_slice(), ["api", "account", ..] | ["api", "users", "me", ..]) {
        return Scope::User;
    }
    match (m.as_str(), segs.as_slice()) {
        ("POST" | "DELETE", ["api", "users", _, "follow"])
        | ("POST" | "DELETE", ["api", "videos", _, "like" | "collect"])
        | ("POST", ["api", "videos", _, "comments"])
        | ("POST" | "DELETE", ["api", "videos", _, "comments", _, "like"])
        | ("POST", ["api", "videos", _, "comments", _, "guest-like"])
        | ("GET", ["api", "videos", _, "comments", "liked" | "state"])
        | ("GET", ["api", "videos", _, "interaction-state" | "guest-liked" | "rating"])
        | ("GET", ["api", "videos", "following"])
        | ("POST", ["api", "videos", _, "feedback" | "guest-comments" | "guest-like" | "rating" | "report"])
        | ("POST", ["api", "ai-wish"])
        | ("POST" | "DELETE", ["api", "posts", _, "like" | "collect"])
        | ("POST", ["api", "posts", _, "comments"])
        | ("POST" | "DELETE", ["api", "posts", _, "comments", _, "like"])
        | ("POST", ["api", "posts", _, "comments", _, "guest-like"])
        | ("POST", ["api", "posts", _, "guest-comments" | "guest-like" | "visit"])
        | ("GET", ["api", "posts", _, "comments", "liked"])
        | ("GET", ["api", "posts", _, "guest-liked"]) => return Scope::User,
        _ => {}
    }

    // 3. Public scope (GET/HEAD only; random sort upgrades to dynamic)
    if m == "GET" {
        let is_public = matches!(
            segs.as_slice(),
            ["api", "videos"]
                | ["api", "videos", "category", _]
                | ["api", "videos", "sort", _]
                | ["api", "videos", "tag", _]
                | ["api", "videos", "user", _]
                | ["api", "videos", "long" | "report-reasons"]
                | ["api", "videos", _, "related"]
                | ["api", "ranks", _]
                | ["api", "posts"]
                | ["api", "posts", "category", _]
                | ["api", "posts", "tag", _]
                | ["api", "posts", "user", _]
                | ["api", "posts", _, "adjacent" | "related"]
                | ["api", "feed", "post", "today"]
                | ["api", "feed", "recommend" | "today"]
                | ["api", "home", "latest" | "recommends"]
                | ["api", "search", "hot" | "suggest"]
                | ["api", "search", "tags", "hot"]
                | ["api", "site", "categories", _]
                | ["api", "site", "config" | "links" | "menus" | "meta" | "pages" | "transit" | "watermark"]
                | ["api", "site", "pages", _]
                | ["api", "site", "sorts", "video"]
                | ["api", "tags"]
                | ["api", "topics"]
                | ["api", "topics", _]
                | ["api", "users", _, "stats"]
        );
        if is_public {
            let has_random_sort = url::form_urlencoded::parse(query.as_bytes()).any(|(k, v)| {
                k == "sort"
                    && (v.trim().eq_ignore_ascii_case("random")
                        || v.trim().eq_ignore_ascii_case("rand"))
            });
            return if has_random_sort {
                Scope::Dynamic
            } else {
                Scope::Public
            };
        }
    }

    Scope::Dynamic
}

struct KeyLen32;
impl KeyType for KeyLen32 {
    fn len(&self) -> usize {
        32
    }
}

pub struct EdgeSession {
    info: String,
    kid: String,
    sid: String,
    clock_offset: i64,
    expires_at: Instant,
    seq: AtomicU32,
    req_key: LessSafeKey,
    res_key: LessSafeKey,
}

impl EdgeSession {
    fn is_usable(&self) -> bool {
        Instant::now() + Duration::from_secs(5) < self.expires_at
            && self.seq.load(Ordering::Relaxed) < MAX_SEQUENCE
    }
}

type Slot = Arc<AsyncMutex<Option<Arc<EdgeSession>>>>;
static SESSIONS: Lazy<StdMutex<HashMap<(String, Scope), Slot>>> =
    Lazy::new(|| StdMutex::new(HashMap::new()));

pub fn clear_sessions() {
    if let Ok(mut map) = SESSIONS.lock() {
        map.clear();
    }
}

fn session_slot(origin: &str, scope: Scope) -> Slot {
    let mut map = SESSIONS.lock().unwrap();
    if map.len() > 64 {
        map.clear();
    }
    map.entry((origin.to_string(), scope))
        .or_insert_with(|| Arc::new(AsyncMutex::new(None)))
        .clone()
}

fn unix_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn encode_b64url(bytes: &[u8]) -> String {
    B64_URL.encode(bytes)
}

pub fn decode_b64url(raw: &str) -> Result<Vec<u8>, String> {
    let trimmed = raw.trim().trim_end_matches('=');
    if trimmed.is_empty()
        || !trimmed
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("加密响应编码无效".into());
    }
    B64_URL
        .decode(trimmed)
        .map_err(|_| "加密响应解码失败".into())
}

pub fn decode_b64_std(raw: &str) -> Result<Vec<u8>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    B64_STD
        .decode(trimmed)
        .or_else(|_| B64_STD_NO_PAD.decode(trimmed.trim_end_matches('=')))
        .map_err(|_| "加密响应正文 Base64 解码失败".into())
}

fn make_nonce(seq: u32) -> [u8; 12] {
    let mut iv = [0u8; 12];
    iv[8..12].copy_from_slice(&seq.to_be_bytes());
    iv
}

fn valid_id(s: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn derive_aead_keys(
    shared: &[u8],
    salt: &[u8],
    info: &str,
) -> Result<(LessSafeKey, LessSafeKey), String> {
    let prk = Salt::new(HKDF_SHA256, salt).extract(shared);
    let mut req_raw = [0u8; 32];
    let mut res_raw = [0u8; 32];

    let req_info = format!("{info}\nreq");
    let res_info = format!("{info}\nres");

    prk.expand(&[req_info.as_bytes()], KeyLen32)
        .and_then(|okm| okm.fill(&mut req_raw))
        .map_err(|_| "HKDF 派生请求密钥失败".to_string())?;
    prk.expand(&[res_info.as_bytes()], KeyLen32)
        .and_then(|okm| okm.fill(&mut res_raw))
        .map_err(|_| "HKDF 派生响应密钥失败".to_string())?;

    let req_key = UnboundKey::new(&AES_256_GCM, &req_raw)
        .map(LessSafeKey::new)
        .map_err(|_| "初始化 AES-GCM 请求密钥失败".to_string())?;
    let res_key = UnboundKey::new(&AES_256_GCM, &res_raw)
        .map(LessSafeKey::new)
        .map_err(|_| "初始化 AES-GCM 响应密钥失败".to_string())?;
    Ok((req_key, res_key))
}

fn build_session_from_hello(
    scope: Scope,
    priv_key: EphemeralPrivateKey,
    hello: &Value,
) -> Result<EdgeSession, String> {
    let v = hello.get("v").and_then(Value::as_i64);
    let typ = hello.get("type").and_then(Value::as_str);
    let kid = hello.get("kid").and_then(Value::as_str).unwrap_or("");
    let sid = hello.get("sid").and_then(Value::as_str).unwrap_or("");
    let site = hello.get("site").and_then(Value::as_str).unwrap_or("");
    let server_time = hello.get("server_time").and_then(Value::as_i64).unwrap_or(0);
    let expires_at = hello.get("expires_at").and_then(Value::as_i64).unwrap_or(0);
    let server_pub_b64 = hello.get("server_pub").and_then(Value::as_str).unwrap_or("");
    let salt_b64 = hello.get("salt").and_then(Value::as_str).unwrap_or("");

    if v != Some(1)
        || typ != Some("hello")
        || !valid_id(kid, 1, 64)
        || !valid_id(sid, 16, 128)
        || site.is_empty()
        || site.len() > 256
        || expires_at <= server_time
        || expires_at - server_time > 600
    {
        return Err("加密握手响应无效".into());
    }

    let salt = decode_b64url(salt_b64)?;
    if salt.len() != 32 {
        return Err("加密握手 salt 长度无效".into());
    }
    let server_pub = decode_b64url(server_pub_b64)?;
    let peer = UnparsedPublicKey::new(&ECDH_P256, &server_pub);

    let info = format!("{PROTOCOL}\n{}\n{site}\n{kid}\n{sid}", scope.as_str());
    let (req_key, res_key) = agree_ephemeral(priv_key, &peer, |shared| {
        derive_aead_keys(shared, &salt, &info)
    })
    .map_err(|_| "ECDH 密钥协商失败".to_string())??;

    let ttl = (expires_at - server_time) as u64;
    Ok(EdgeSession {
        info,
        kid: kid.to_string(),
        sid: sid.to_string(),
        clock_offset: server_time - unix_secs(),
        expires_at: Instant::now() + Duration::from_secs(ttl),
        seq: AtomicU32::new(0),
        req_key,
        res_key,
    })
}

async fn open_session(
    origin: &str,
    scope: Scope,
    timeout: Duration,
) -> Result<(Arc<EdgeSession>, crate::net::RawHttpMeta), String> {
    let rng = SystemRandom::new();
    let priv_key = EphemeralPrivateKey::generate(&ECDH_P256, &rng)
        .map_err(|_| "生成 P-256 临时密钥对失败".to_string())?;
    let pub_key = priv_key
        .compute_public_key()
        .map_err(|_| "计算 P-256 公钥失败".to_string())?;
    let client_pub = encode_b64url(pub_key.as_ref());

    let url = format!("{origin}{}", scope.endpoint());
    let referer = format!("{origin}/");
    let hello_body = json!({
        "v": 1,
        "type": "hello",
        "client_pub": client_pub,
    });

    let raw = crate::net::post_json_raw(&url, timeout, &hello_body, &[], &referer).await?;
    if raw.meta.status == 401 || raw.meta.status == 403 {
        return Err("401".to_string());
    }
    if !(200..300).contains(&raw.meta.status) {
        return Err(format!(
            "HTTP {}: {}",
            raw.meta.status,
            raw.text.chars().take(160).collect::<String>()
        ));
    }
    let hello: Value = serde_json::from_str(&raw.text)
        .map_err(|e| format!("加密握手解析失败: {e}"))?;
    let sess = build_session_from_hello(scope, priv_key, &hello)?;
    Ok((Arc::new(sess), raw.meta))
}

async fn get_session(origin: &str, scope: Scope) -> Result<Arc<EdgeSession>, String> {
    let slot = session_slot(origin, scope);
    let mut guard = slot.lock().await;
    if let Some(existing) = guard.as_ref() {
        if existing.is_usable() {
            return Ok(existing.clone());
        }
    }
    let (fresh, _) = open_session(origin, scope, HANDSHAKE_TIMEOUT).await?;
    *guard = Some(fresh.clone());
    Ok(fresh)
}

async fn evict_session(origin: &str, scope: Scope, target: &Arc<EdgeSession>) {
    let slot = session_slot(origin, scope);
    let mut guard = slot.lock().await;
    if guard.as_ref().is_some_and(|cur| Arc::ptr_eq(cur, target)) {
        *guard = None;
    }
}

/// 实测接口握手可用性（取代仅 `GET /` 导致静态导航页/503 API 误报绿灯）。
pub async fn probe_hello(origin: &str, timeout: Duration) -> Result<u64, (Option<u16>, String)> {
    let started = Instant::now();
    match open_session(origin, Scope::Public, timeout).await {
        Ok((_sess, meta)) => {
            let _ = meta;
            Ok(started.elapsed().as_millis() as u64)
        }
        Err(e) => {
            let status = e
                .strip_prefix("HTTP ")
                .and_then(|s| s.split(|c: char| !c.is_ascii_digit()).next())
                .and_then(|n| n.parse::<u16>().ok())
                .or_else(|| if e == "401" { Some(401) } else { None });
            Err((status, e))
        }
    }
}

pub struct EncryptedResponse {
    pub status: u16,
    pub body: String,
    pub meta: crate::net::RawHttpMeta,
}

fn sort_public_path(raw_path: &str) -> String {
    let Some((pathname, query)) = raw_path.split_once('?') else {
        return raw_path.to_string();
    };
    let mut pairs: Vec<(String, String)> = url::form_urlencoded::parse(query.as_bytes())
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    // 稳定排序（保留同名参数原相对顺序），与 edge-cache-runtime.js 一致。
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    if pairs.is_empty() {
        return pathname.to_string();
    }
    let mut ser = url::form_urlencoded::Serializer::new(String::new());
    ser.extend_pairs(pairs);
    format!("{pathname}?{}", ser.finish())
}

async fn try_public_json(
    origin: &str,
    method: &str,
    path: &str,
    encoded_body: &str,
    ctype: &str,
) -> Option<Result<EncryptedResponse, String>> {
    let sorted_path = sort_public_path(path);
    let url = format!("{origin}{}", Scope::Public.endpoint());
    let referer = format!("{origin}/");
    let packet = json!({
        "method": method,
        "path": sorted_path,
        "body": encoded_body,
        "content_type": ctype,
    });
    let raw = match crate::net::post_json_raw(
        &url,
        Duration::from_secs(20),
        &packet,
        &[("X-HG-Public-JSON", "1")],
        &referer,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => return Some(Err(e)),
    };

    if (200..300).contains(&raw.meta.status) {
        if raw.text.trim().is_empty() {
            return Some(Ok(EncryptedResponse {
                status: raw.meta.status,
                body: raw.text,
                meta: raw.meta,
            }));
        }
        if let Ok(parsed) = serde_json::from_str::<Value>(&raw.text) {
            if parsed.get("status").is_some() {
                return Some(Ok(EncryptedResponse {
                    status: raw.meta.status,
                    body: raw.text,
                    meta: raw.meta,
                }));
            }
        }
    }

    // 若服务端要求必须走加密会话（400/404/403/503 或响应非业务 JSON），回退到 ECDH 加密通道。
    None
}

async fn call_encrypted_once(
    origin: &str,
    scope: Scope,
    session: &EdgeSession,
    method: &str,
    path: &str,
    encoded_body: &str,
    ctype: &str,
    token: Option<&str>,
) -> Result<(EncryptedResponse, bool), String> {
    let seq = session.seq.fetch_add(1, Ordering::Relaxed) + 1;
    if seq > MAX_SEQUENCE {
        return Err("加密会话序号已耗尽".into());
    }
    let ts = unix_secs() + session.clock_offset;
    let iv = make_nonce(seq);

    let mut packet = json!({
        "request": {
            "method": method,
            "path": path,
            "body": encoded_body,
            "content_type": ctype,
        }
    });
    if scope != Scope::Public {
        if let Some(t) = token {
            if !t.is_empty() {
                packet["authorization"] = json!(format!("Bearer {t}"));
            }
        }
    }

    let mut buf = serde_json::to_vec(&packet).map_err(|e| e.to_string())?;
    let aad_req = format!("{}\nreq\n{seq}\n{ts}", session.info);
    session
        .req_key
        .seal_in_place_append_tag(
            Nonce::assume_unique_for_key(iv),
            Aad::from(aad_req.as_bytes()),
            &mut buf,
        )
        .map_err(|_| "AES-GCM 加密失败".to_string())?;

    let envelope = json!({
        "v": 1,
        "type": "call",
        "kid": session.kid,
        "sid": session.sid,
        "seq": seq,
        "ts": ts,
        "ciphertext": encode_b64url(&buf),
    });

    let url = format!("{origin}{}", scope.endpoint());
    let referer = format!("{origin}/");
    let raw = crate::net::post_json_raw(&url, Duration::from_secs(20), &envelope, &[], &referer)
        .await?;

    if !(200..300).contains(&raw.meta.status) {
        if let Ok(err_json) = serde_json::from_str::<Value>(&raw.text) {
            if err_json.get("error").and_then(Value::as_str) == Some("session_expired") {
                return Ok((
                    EncryptedResponse {
                        status: raw.meta.status,
                        body: raw.text,
                        meta: raw.meta,
                    },
                    true,
                ));
            }
        }
        if raw.meta.status == 401 || raw.meta.status == 403 {
            return Err("401".to_string());
        }
        return Err(format!(
            "HTTP {}: {}",
            raw.meta.status,
            raw.text.chars().take(160).collect::<String>()
        ));
    }

    let result: Value =
        serde_json::from_str(&raw.text).map_err(|e| format!("加密外层响应解析失败: {e}"))?;
    if result.get("v").and_then(Value::as_i64) != Some(1)
        || result.get("type").and_then(Value::as_str) != Some("result")
        || result.get("sid").and_then(Value::as_str) != Some(session.sid.as_str())
        || result.get("kid").and_then(Value::as_str) != Some(session.kid.as_str())
        || result.get("seq").and_then(Value::as_i64) != Some(seq as i64)
        || result.get("ts").and_then(Value::as_i64) != Some(ts)
    {
        return Err("加密接口响应不匹配".into());
    }

    let ct_b64 = result
        .get("ciphertext")
        .and_then(Value::as_str)
        .ok_or("加密响应缺少 ciphertext")?;
    let mut pt = decode_b64url(ct_b64)?;
    let aad_res = format!("{}\nres\n{seq}\n{ts}", session.info);
    let plain = session
        .res_key
        .open_in_place(
            Nonce::assume_unique_for_key(iv),
            Aad::from(aad_res.as_bytes()),
            &mut pt,
        )
        .map_err(|_| "AES-GCM 解密响应失败".to_string())?;

    let inner: Value =
        serde_json::from_slice(plain).map_err(|e| format!("加密内层 JSON 解析失败: {e}"))?;
    let status = inner
        .get("status")
        .and_then(Value::as_u64)
        .filter(|&s| (200..=599).contains(&s))
        .ok_or("加密内层状态码无效")? as u16;

    let no_body = method.eq_ignore_ascii_case("HEAD") || matches!(status, 204 | 205 | 304);
    let decoded_body = if no_body {
        String::new()
    } else {
        let b64 = inner.get("body").and_then(Value::as_str).unwrap_or("");
        let bytes = decode_b64_std(b64)?;
        String::from_utf8(bytes).map_err(|_| "加密内层响应不是有效 UTF-8".to_string())?
    };

    let mut meta = raw.meta;
    meta.status = status;
    Ok((
        EncryptedResponse {
            status,
            body: decoded_body,
            meta,
        },
        false,
    ))
}

/// 通过 `dx150-edge-v1` 网关发起业务请求（`public` 先试快速 JSON，必要时回退 ECDH 加密）。
pub async fn send(
    origin: &str,
    scope: Scope,
    method: &str,
    path: &str,
    encoded_body: &str,
    ctype: &str,
    token: Option<&str>,
) -> Result<EncryptedResponse, String> {
    if scope == Scope::Public && token.is_none() {
        if let Some(res) = try_public_json(origin, method, path, encoded_body, ctype).await {
            return res;
        }
    }

    for retry in 0..2 {
        let session = get_session(origin, scope).await?;
        let (resp, expired) = call_encrypted_once(
            origin,
            scope,
            &session,
            method,
            path,
            encoded_body,
            ctype,
            token,
        )
        .await?;
        if !expired {
            return Ok(resp);
        }
        evict_session(origin, scope, &session).await;
        if retry == 1 {
            return Err("加密会话已过期，请重试".into());
        }
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_scope_matches_upstream_gateway_rules() {
        // Playback
        assert_eq!(
            route_scope("POST", "/api/videos/12/episodes/1/playback"),
            Scope::Playback
        );
        assert_eq!(route_scope("GET", "/api/videos/12/play?ep=1"), Scope::Playback);
        assert_eq!(route_scope("POST", "/api/posts/5/playback"), Scope::Playback);

        // User
        assert_eq!(
            route_scope("POST", "/api/account/loginByPassword"),
            Scope::User
        );
        assert_eq!(route_scope("GET", "/api/account/me"), Scope::User);
        assert_eq!(route_scope("GET", "/api/users/me/history"), Scope::User);
        assert_eq!(route_scope("POST", "/api/videos/12/comments?ep=1"), Scope::User);
        assert_eq!(route_scope("POST", "/api/videos/12/like?ep=1"), Scope::User);
        assert_eq!(route_scope("DELETE", "/api/videos/12/collect"), Scope::User);
        assert_eq!(route_scope("POST", "/api/users/99/follow"), Scope::User);
        assert_eq!(route_scope("POST", "/api/posts/5/like"), Scope::User);

        // Public & dynamic sort upgrade
        assert_eq!(route_scope("GET", "/api/videos?page=1&size=20"), Scope::Public);
        assert_eq!(
            route_scope("GET", "/api/videos?page=1&sort=random"),
            Scope::Dynamic
        );
        assert_eq!(
            route_scope("GET", "/api/videos/category/ai-duanju?page=1&size=20"),
            Scope::Public
        );
        assert_eq!(route_scope("GET", "/api/ranks/hot"), Scope::Public);
        assert_eq!(route_scope("GET", "/api/posts?page=1&size=20"), Scope::Public);

        // Dynamic
        assert_eq!(route_scope("GET", "/api/videos/12"), Scope::Dynamic);
        assert_eq!(
            route_scope("GET", "/api/videos/12/comments?ep=1&page=1"),
            Scope::Dynamic
        );
        assert_eq!(route_scope("GET", "/api/posts/5"), Scope::Dynamic);
        assert_eq!(
            route_scope("GET", "/api/posts/5/comments?page=1&size=20"),
            Scope::Dynamic
        );
        assert_eq!(
            route_scope("GET", "/api/search?q=test&page=1&size=20"),
            Scope::Dynamic
        );
    }

    #[test]
    fn public_query_params_sort_stably() {
        assert_eq!(
            sort_public_path("/api/videos?size=20&page=1&tag=b&tag=a"),
            "/api/videos?page=1&size=20&tag=b&tag=a"
        );
        assert_eq!(sort_public_path("/api/ranks/hot"), "/api/ranks/hot");
    }

    #[test]
    fn ecdh_hkdf_aes_gcm_roundtrip_matches_protocol() {
        let rng = SystemRandom::new();
        let client_priv = EphemeralPrivateKey::generate(&ECDH_P256, &rng).unwrap();
        let server_priv = EphemeralPrivateKey::generate(&ECDH_P256, &rng).unwrap();
        let server_pub = server_priv.compute_public_key().unwrap();
        let client_pub = client_priv.compute_public_key().unwrap();

        let salt = [0x42u8; 32];
        let now = unix_secs();
        let hello = json!({
            "v": 1,
            "type": "hello",
            "kid": "k1",
            "sid": "session_1234567890abcd",
            "site": "huangguoai.com",
            "server_time": now,
            "expires_at": now + 300,
            "server_pub": encode_b64url(server_pub.as_ref()),
            "salt": encode_b64url(&salt),
        });

        let session = build_session_from_hello(Scope::Dynamic, client_priv, &hello).unwrap();

        // Server side derives the exact same pair of keys from client_pub
        let (srv_req_key, srv_res_key) = agree_ephemeral(
            server_priv,
            &UnparsedPublicKey::new(&ECDH_P256, client_pub.as_ref()),
            |shared| derive_aead_keys(shared, &salt, &session.info),
        )
        .unwrap()
        .unwrap();

        let iv = make_nonce(1);
        let ts = now;
        let aad_req = format!("{}\nreq\n1\n{ts}", session.info);
        let mut req_buf = br#"{"request":{"method":"GET","path":"/api/videos/12"}}"#.to_vec();
        session
            .req_key
            .seal_in_place_append_tag(
                Nonce::assume_unique_for_key(iv),
                Aad::from(aad_req.as_bytes()),
                &mut req_buf,
            )
            .unwrap();
        let dec_req = srv_req_key
            .open_in_place(
                Nonce::assume_unique_for_key(iv),
                Aad::from(aad_req.as_bytes()),
                &mut req_buf,
            )
            .unwrap();
        assert!(std::str::from_utf8(dec_req)
            .unwrap()
            .contains("/api/videos/12"));

        let aad_res = format!("{}\nres\n1\n{ts}", session.info);
        let inner = json!({
            "status": 200,
            "headers": {"content-type": ["application/json"]},
            "body": B64_STD.encode(r#"{"status":1,"data":{"ok":true}}"#),
        });
        let mut res_buf = serde_json::to_vec(&inner).unwrap();
        srv_res_key
            .seal_in_place_append_tag(
                Nonce::assume_unique_for_key(iv),
                Aad::from(aad_res.as_bytes()),
                &mut res_buf,
            )
            .unwrap();
        let dec_res = session
            .res_key
            .open_in_place(
                Nonce::assume_unique_for_key(iv),
                Aad::from(aad_res.as_bytes()),
                &mut res_buf,
            )
            .unwrap();
        let parsed: Value = serde_json::from_slice(dec_res).unwrap();
        let decoded = String::from_utf8(decode_b64_std(parsed["body"].as_str().unwrap()).unwrap())
            .unwrap();
        assert_eq!(decoded, r#"{"status":1,"data":{"ok":true}}"#);
    }

    /// 手动跑：`cargo test --lib -- --ignored edge_crypto::tests::live_gateway_smoke -- --nocapture`
    #[tokio::test]
    #[ignore = "联网测试：验证 dx150-edge-v1 固定线路与镜像接口/封面/播放全链路"]
    async fn live_gateway_smoke() {
        let mirrors = crate::lines::mirrors().await.expect("mirrors");
        let mirror_url = mirrors["items"][0]["url"]
            .as_str()
            .expect("first mirror")
            .to_string();

        // 验证线路探测（主站 ok=true，静态导航页 huangguoai.ai 必须 ok=false）
        let probed = crate::lines::probe(vec![
            crate::net::DEFAULT_BASE.to_string(),
            "https://huangguoai.ai".to_string(),
            mirror_url.clone(),
        ])
        .await;
        println!("probe={probed}");
        assert_eq!(probed[0]["ok"], true);
        assert_eq!(probed[1]["ok"], false);
        assert_eq!(probed[2]["ok"], true);

        for origin in [crate::net::DEFAULT_BASE.to_string(), mirror_url] {
            let feed = crate::net::edge(&origin, "GET", "/api/videos?page=1&size=2", None, None)
                .await
                .expect("feed");
            let first = &feed["items"][0];
            let vid = first["id"].as_i64().expect("video id");
            let cover = first["cover"].as_str().expect("cover url");
            let detail = crate::net::edge(&origin, "GET", &format!("/api/videos/{vid}"), None, None)
                .await
                .expect("detail");
            assert!(detail["title"].as_str().is_some_and(|s| !s.is_empty()));
            let play = crate::net::edge(
                &origin,
                "POST",
                &format!("/api/videos/{vid}/episodes/1/playback"),
                Some(json!({})),
                None,
            )
            .await
            .expect("playback");
            let stream = play["streams"]["main"]
                .as_str()
                .expect("streams.main");
            assert!(stream.contains(".m3u8"));
            let (kind, img_bytes) = crate::media::load_image(cover).await.expect("load_image");
            assert!(kind.starts_with("image/") && img_bytes.len() > 1000);
            println!("origin={origin} vid={vid} kind={kind} stream_ok=true");
        }
    }
}
