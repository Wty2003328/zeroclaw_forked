use super::traits::{Channel, ChannelMessage, SendMessage};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// WeCom (WeChat Enterprise) channel — supports both Bot Webhook mode and
/// Enterprise App mode with full bidirectional messaging.
///
/// **Bot Webhook mode** (legacy): send-only via `webhook_key`.
/// **Enterprise App mode**: send via WeCom Message API, receive via gateway
/// callback URL with AES-CBC encrypted XML messages.
pub struct WeComChannel {
    /// Enterprise App fields (None in webhook-only mode)
    enterprise: Option<EnterpriseApp>,
    /// Legacy bot webhook key (None in enterprise mode)
    webhook_key: Option<String>,
    /// Allowed user IDs. Empty = deny all, "*" = allow all.
    allowed_users: Vec<String>,
}

struct EnterpriseApp {
    corp_id: String,
    corp_secret: String,
    agent_id: i64,
    token: String,
    aes_key: [u8; 32],
    aes_iv: [u8; 16],
    access_token: Arc<RwLock<Option<CachedAccessToken>>>,
}

#[derive(Debug, Clone)]
struct CachedAccessToken {
    value: String,
    refresh_after: Instant,
}

impl WeComChannel {
    /// Create an Enterprise App channel with full bidirectional messaging.
    pub fn new_enterprise(
        corp_id: String,
        corp_secret: String,
        agent_id: i64,
        token: String,
        encoding_aes_key: String,
        allowed_users: Vec<String>,
    ) -> anyhow::Result<Self> {
        let (aes_key, aes_iv) = crypto::decode_aes_key(&encoding_aes_key)?;
        Ok(Self {
            enterprise: Some(EnterpriseApp {
                corp_id,
                corp_secret,
                agent_id,
                token,
                aes_key,
                aes_iv,
                access_token: Arc::new(RwLock::new(None)),
            }),
            webhook_key: None,
            allowed_users,
        })
    }

    /// Create a legacy Bot Webhook channel (send-only).
    pub fn new_webhook(webhook_key: String, allowed_users: Vec<String>) -> Self {
        Self {
            enterprise: None,
            webhook_key: Some(webhook_key),
            allowed_users,
        }
    }

    /// Backward-compatible constructor (used by existing code).
    pub fn new(webhook_key: String, allowed_users: Vec<String>) -> Self {
        Self::new_webhook(webhook_key, allowed_users)
    }

    fn http_client(&self) -> reqwest::Client {
        crate::config::build_runtime_proxy_client("channel.wecom")
    }

    fn is_user_allowed(&self, user_id: &str) -> bool {
        self.allowed_users.iter().any(|u| u == "*" || u == user_id)
    }

    /// Verify WeCom callback URL (GET request from WeCom server).
    /// Returns the decrypted echostr on success.
    pub fn verify_callback(
        &self,
        msg_signature: &str,
        timestamp: &str,
        nonce: &str,
        echostr: &str,
    ) -> Option<String> {
        let ent = self.enterprise.as_ref()?;
        if !crypto::verify_signature(&ent.token, timestamp, nonce, echostr, msg_signature) {
            tracing::warn!("WeCom callback verification failed — signature mismatch");
            return None;
        }
        match crypto::decrypt_message(&ent.aes_key, &ent.aes_iv, echostr) {
            Ok((content, _corp_id)) => Some(content),
            Err(e) => {
                tracing::warn!("WeCom callback decrypt failed: {e}");
                None
            }
        }
    }

    /// Parse an incoming encrypted message POST from WeCom.
    /// Returns parsed `ChannelMessage`s or None on verification/parse failure.
    pub fn parse_encrypted_message(
        &self,
        msg_signature: &str,
        timestamp: &str,
        nonce: &str,
        body_xml: &str,
    ) -> Option<Vec<ChannelMessage>> {
        let ent = self.enterprise.as_ref()?;

        // Extract <Encrypt> from the outer XML envelope
        let encrypt_content = match xml::extract_element(body_xml, "Encrypt") {
            Some(v) => v,
            None => {
                tracing::warn!("WeCom: failed to extract <Encrypt> from body (len={})", body_xml.len());
                return None;
            }
        };

        // Verify signature against the encrypted content
        if !crypto::verify_signature(
            &ent.token,
            timestamp,
            nonce,
            &encrypt_content,
            msg_signature,
        ) {
            tracing::warn!("WeCom message signature verification failed");
            return None;
        }

        // Decrypt
        let (decrypted_xml, _corp_id) = match crypto::decrypt_message(&ent.aes_key, &ent.aes_iv, &encrypt_content) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("WeCom message decrypt error: {e}");
                return None;
            }
        };

        // Parse the decrypted inner XML
        let msg = xml::parse_message_xml(&decrypted_xml)?;

        // Check user allowlist
        if !self.is_user_allowed(&msg.from_user) {
            tracing::warn!("WeCom message from non-allowed user: {}", msg.from_user);
            return None;
        }

        let content = match msg.msg_type.as_str() {
            "text" => msg.content.unwrap_or_default(),
            "voice" => msg.recognition.unwrap_or_else(|| {
                format!("[voice message: {}]", msg.media_id.unwrap_or_default())
            }),
            "image" => format!("[image: {}]", msg.media_id.unwrap_or_default()),
            "video" => format!("[video: {}]", msg.media_id.unwrap_or_default()),
            "location" => format!(
                "[location: {}, {}]",
                msg.location_x.unwrap_or_default(),
                msg.location_y.unwrap_or_default()
            ),
            other => format!("[{other} message]"),
        };

        if content.is_empty() {
            return None;
        }

        let ts = msg
            .create_time
            .parse::<u64>()
            .unwrap_or_else(|_| std::time::UNIX_EPOCH.elapsed().unwrap_or_default().as_secs());

        Some(vec![ChannelMessage {
            id: msg.msg_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            sender: msg.from_user.clone(),
            reply_target: msg.from_user,
            content,
            channel: "wecom".to_string(),
            timestamp: ts,
            thread_ts: None,
            interruption_scope_id: None,
            attachments: vec![],
        }])
    }

    /// Get a valid access token, refreshing if needed.
    async fn get_access_token(&self) -> anyhow::Result<String> {
        let ent = self
            .enterprise
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not in enterprise app mode"))?;

        // Check cache
        {
            let guard = ent.access_token.read().await;
            if let Some(ref cached) = *guard {
                if Instant::now() < cached.refresh_after {
                    return Ok(cached.value.clone());
                }
            }
        }

        // Fetch new token
        let url = format!(
            "https://qyapi.weixin.qq.com/cgi-bin/gettoken?corpid={}&corpsecret={}",
            ent.corp_id, ent.corp_secret
        );

        let resp: serde_json::Value = self.http_client().get(&url).send().await?.json().await?;

        let errcode = resp.get("errcode").and_then(|v| v.as_i64()).unwrap_or(-1);
        if errcode != 0 {
            let errmsg = resp
                .get("errmsg")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            anyhow::bail!("WeCom gettoken failed (errcode={errcode}): {errmsg}");
        }

        let token = resp
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing access_token in response"))?
            .to_string();

        let expires_in = resp
            .get("expires_in")
            .and_then(|v| v.as_u64())
            .unwrap_or(7200);

        // Cache with 5-minute safety margin
        let refresh_after =
            Instant::now() + std::time::Duration::from_secs(expires_in.saturating_sub(300));
        let mut guard = ent.access_token.write().await;
        *guard = Some(CachedAccessToken {
            value: token.clone(),
            refresh_after,
        });

        tracing::info!("WeCom access token refreshed (expires in {expires_in}s)");
        Ok(token)
    }

    /// Strip markdown formatting for plain-text channels like WeCom.
    fn strip_markdown(text: &str) -> String {
        let mut result = String::with_capacity(text.len());
        let mut chars = text.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                // Strip bold/italic markers
                '*' | '_' => {
                    // Consume consecutive markers (**, ***, __, ___)
                    while chars.peek() == Some(&ch) {
                        chars.next();
                    }
                }
                // Strip heading markers at line start
                '#' if result.is_empty() || result.ends_with('\n') => {
                    while chars.peek() == Some(&'#') {
                        chars.next();
                    }
                    // Skip the space after ###
                    if chars.peek() == Some(&' ') {
                        chars.next();
                    }
                }
                // Strip inline code backticks
                '`' => {
                    // Skip ``` (code fences) or single `
                    while chars.peek() == Some(&'`') {
                        chars.next();
                    }
                    // For code fences (```lang), skip until newline
                    if result.ends_with('\n') || result.is_empty() {
                        while chars.peek().is_some_and(|c| *c != '\n') {
                            let c = chars.next().unwrap();
                            // If this looks like inline content, keep it
                            if c == ' ' || c.is_alphanumeric() {
                                // This was a code fence language tag, skip it
                                while chars.peek().is_some_and(|c| *c != '\n') {
                                    chars.next();
                                }
                                break;
                            }
                        }
                    }
                }
                // Convert [text](url) → text
                '[' => {
                    let mut link_text = String::new();
                    let mut found_close = false;
                    for c in chars.by_ref() {
                        if c == ']' {
                            found_close = true;
                            break;
                        }
                        link_text.push(c);
                    }
                    if found_close && chars.peek() == Some(&'(') {
                        chars.next(); // skip (
                        // skip url until )
                        for c in chars.by_ref() {
                            if c == ')' {
                                break;
                            }
                        }
                        result.push_str(&link_text);
                    } else {
                        result.push('[');
                        result.push_str(&link_text);
                        if found_close {
                            result.push(']');
                        }
                    }
                }
                _ => result.push(ch),
            }
        }
        result
    }

    /// Send via legacy bot webhook.
    async fn send_via_webhook(
        &self,
        webhook_key: &str,
        message: &SendMessage,
    ) -> anyhow::Result<()> {
        let url = format!(
            "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key={webhook_key}"
        );
        let plain = Self::strip_markdown(&message.content);
        let body = serde_json::json!({
            "msgtype": "text",
            "text": { "content": plain }
        });

        let resp = self.http_client().post(&url).json(&body).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let err = resp.text().await.unwrap_or_default();
            anyhow::bail!("WeCom webhook send failed ({status}): {err}");
        }

        let result: serde_json::Value = resp.json().await?;
        let errcode = result.get("errcode").and_then(|v| v.as_i64()).unwrap_or(-1);
        if errcode != 0 {
            let errmsg = result
                .get("errmsg")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            anyhow::bail!("WeCom webhook error (errcode={errcode}): {errmsg}");
        }
        Ok(())
    }

    /// Send via Enterprise App message API.
    async fn send_via_app(&self, message: &SendMessage) -> anyhow::Result<()> {
        let ent = self
            .enterprise
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not in enterprise app mode"))?;

        let token = self.get_access_token().await?;
        let url = format!(
            "https://qyapi.weixin.qq.com/cgi-bin/message/send?access_token={token}"
        );

        let plain = Self::strip_markdown(&message.content);
        let body = serde_json::json!({
            "touser": message.recipient,
            "msgtype": "text",
            "agentid": ent.agent_id,
            "text": { "content": plain }
        });

        let resp = self.http_client().post(&url).json(&body).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let err = resp.text().await.unwrap_or_default();
            anyhow::bail!("WeCom app send failed ({status}): {err}");
        }

        let result: serde_json::Value = resp.json().await?;
        let errcode = result.get("errcode").and_then(|v| v.as_i64()).unwrap_or(-1);
        if errcode != 0 {
            let errmsg = result
                .get("errmsg")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            anyhow::bail!("WeCom app send error (errcode={errcode}): {errmsg}");
        }
        Ok(())
    }
}

#[async_trait]
impl Channel for WeComChannel {
    fn name(&self) -> &str {
        "wecom"
    }

    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        if self.enterprise.is_some() {
            self.send_via_app(message).await
        } else if let Some(ref key) = self.webhook_key {
            self.send_via_webhook(key, message).await
        } else {
            anyhow::bail!("WeCom channel not configured for sending")
        }
    }

    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        if self.enterprise.is_some() {
            tracing::info!("WeCom: Enterprise App channel ready (receive via gateway /wecom)");
        } else {
            tracing::info!("WeCom: Bot Webhook channel ready (send-only)");
        }
        tx.closed().await;
        Ok(())
    }

    async fn health_check(&self) -> bool {
        if self.enterprise.is_some() {
            self.get_access_token().await.is_ok()
        } else if let Some(ref key) = self.webhook_key {
            let url = format!(
                "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key={key}"
            );
            self.http_client()
                .post(&url)
                .json(&serde_json::json!({"msgtype":"text","text":{"content":"health_check"}}))
                .send()
                .await
                .is_ok_and(|r| r.status().is_success())
        } else {
            false
        }
    }
}

/// WeCom message encryption/decryption (AES-256-CBC + SHA1 signature).
pub mod crypto {
    use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
    use base64::{
        engine::general_purpose::{GeneralPurpose, GeneralPurposeConfig, STANDARD as BASE64},
        alphabet, engine::DecodePaddingMode, Engine,
    };

    type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
    type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

    /// Lenient base64 engine for WeCom's EncodingAESKey.
    /// WeCom keys have non-zero trailing bits which the strict STANDARD
    /// engine rejects. We must allow both trailing bits and flexible padding.
    const LENIENT_BASE64: GeneralPurpose = GeneralPurpose::new(
        &alphabet::STANDARD,
        GeneralPurposeConfig::new()
            .with_decode_allow_trailing_bits(true)
            .with_decode_padding_mode(DecodePaddingMode::Indifferent),
    );

    /// Decode the 43-character EncodingAESKey to a 32-byte AES key.
    /// The IV is the first 16 bytes of the key.
    pub fn decode_aes_key(encoding_aes_key: &str) -> anyhow::Result<([u8; 32], [u8; 16])> {
        let padded = format!("{encoding_aes_key}=");
        let decoded = LENIENT_BASE64.decode(&padded)?;
        if decoded.len() != 32 {
            anyhow::bail!(
                "EncodingAESKey decoded to {} bytes, expected 32",
                decoded.len()
            );
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&decoded);
        let mut iv = [0u8; 16];
        iv.copy_from_slice(&key[..16]);
        Ok((key, iv))
    }

    /// Verify WeCom callback signature.
    pub fn verify_signature(
        token: &str,
        timestamp: &str,
        nonce: &str,
        encrypt_msg: &str,
        signature: &str,
    ) -> bool {
        let computed = generate_signature(token, timestamp, nonce, encrypt_msg);
        use subtle::ConstantTimeEq;
        computed.as_bytes().ct_eq(signature.as_bytes()).into()
    }

    /// Generate WeCom signature for a set of parameters.
    pub fn generate_signature(
        token: &str,
        timestamp: &str,
        nonce: &str,
        encrypt_msg: &str,
    ) -> String {
        use sha1::{Digest, Sha1};
        let mut parts = [token, timestamp, nonce, encrypt_msg];
        parts.sort();
        let combined = parts.join("");
        let hash = Sha1::digest(combined.as_bytes());
        hex::encode(hash)
    }

    /// Decrypt a WeCom AES-CBC encrypted message.
    /// Returns (content, corp_id).
    ///
    /// WeCom uses PKCS#7 padding with block size 32 (not 16), so we decrypt
    /// without automatic unpadding and strip the padding manually.
    pub fn decrypt_message(
        aes_key: &[u8; 32],
        iv: &[u8; 16],
        encrypted_b64: &str,
    ) -> anyhow::Result<(String, String)> {
        use aes::cipher::block_padding::NoPadding;

        let ciphertext = LENIENT_BASE64.decode(encrypted_b64)?;
        let mut buf = ciphertext.clone();
        let plaintext = Aes256CbcDec::new(aes_key.into(), iv.into())
            .decrypt_padded_mut::<NoPadding>(&mut buf)
            .map_err(|e| anyhow::anyhow!("AES decrypt failed: {e}"))?;

        // Strip WeCom's PKCS#7 padding (block size 32, so pad value can be 1..=32)
        if plaintext.is_empty() {
            anyhow::bail!("Decrypted data is empty");
        }
        let pad_byte = *plaintext.last().unwrap() as usize;
        if pad_byte == 0 || pad_byte > 32 || pad_byte > plaintext.len() {
            anyhow::bail!("Invalid PKCS7 padding value: {pad_byte}");
        }
        let unpadded = &plaintext[..plaintext.len() - pad_byte];

        if unpadded.len() < 20 {
            anyhow::bail!("Decrypted data too short ({} bytes)", unpadded.len());
        }

        // Layout: 16 random bytes | 4-byte content length (big-endian) | content | corp_id
        let content_len =
            u32::from_be_bytes([unpadded[16], unpadded[17], unpadded[18], unpadded[19]])
                as usize;
        let content_start = 20;
        let content_end = content_start + content_len;
        if content_end > unpadded.len() {
            anyhow::bail!(
                "Content length {content_len} exceeds available data {}",
                unpadded.len() - 20
            );
        }

        let content = String::from_utf8(unpadded[content_start..content_end].to_vec())?;
        let corp_id = String::from_utf8(unpadded[content_end..].to_vec())?;
        Ok((content, corp_id))
    }

    /// Encrypt a message for WeCom reply.
    pub fn encrypt_message(
        aes_key: &[u8; 32],
        iv: &[u8; 16],
        content: &str,
        corp_id: &str,
    ) -> anyhow::Result<String> {
        let content_bytes = content.as_bytes();
        let corp_id_bytes = corp_id.as_bytes();
        let content_len = (content_bytes.len() as u32).to_be_bytes();

        let mut plaintext = Vec::with_capacity(20 + content_bytes.len() + corp_id_bytes.len());
        let mut random_bytes = [0u8; 16];
        rand::fill(&mut random_bytes);
        plaintext.extend_from_slice(&random_bytes);
        plaintext.extend_from_slice(&content_len);
        plaintext.extend_from_slice(content_bytes);
        plaintext.extend_from_slice(corp_id_bytes);

        let ciphertext = Aes256CbcEnc::new(aes_key.into(), iv.into())
            .encrypt_padded_vec_mut::<Pkcs7>(&plaintext);

        Ok(BASE64.encode(&ciphertext))
    }
}

/// XML parsing helpers for WeCom message format.
mod xml {
    pub struct WeComMessage {
        pub from_user: String,
        pub create_time: String,
        pub msg_type: String,
        pub content: Option<String>,
        pub msg_id: Option<String>,
        pub media_id: Option<String>,
        pub recognition: Option<String>,
        pub location_x: Option<String>,
        pub location_y: Option<String>,
    }

    /// Extract a top-level XML element's text content by tag name.
    pub fn extract_element(xml_str: &str, tag: &str) -> Option<String> {
        let open = format!("<{tag}>");
        let close = format!("</{tag}>");
        let start = xml_str.find(&open)? + open.len();
        let end = xml_str.find(&close)?;
        let inner = &xml_str[start..end];
        let content = inner
            .trim()
            .strip_prefix("<![CDATA[")
            .and_then(|s| s.strip_suffix("]]>"))
            .unwrap_or(inner.trim());
        Some(content.to_string())
    }

    /// Parse the decrypted WeCom XML message into structured fields.
    pub fn parse_message_xml(xml_str: &str) -> Option<WeComMessage> {
        Some(WeComMessage {
            from_user: extract_element(xml_str, "FromUserName")?,
            create_time: extract_element(xml_str, "CreateTime").unwrap_or_default(),
            msg_type: extract_element(xml_str, "MsgType").unwrap_or_else(|| "text".to_string()),
            content: extract_element(xml_str, "Content"),
            msg_id: extract_element(xml_str, "MsgId"),
            media_id: extract_element(xml_str, "MediaId"),
            recognition: extract_element(xml_str, "Recognition"),
            location_x: extract_element(xml_str, "Location_X"),
            location_y: extract_element(xml_str, "Location_Y"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        let ch = WeComChannel::new("test-key".into(), vec![]);
        assert_eq!(ch.name(), "wecom");
    }

    #[test]
    fn test_user_allowed_wildcard() {
        let ch = WeComChannel::new("key".into(), vec!["*".into()]);
        assert!(ch.is_user_allowed("anyone"));
    }

    #[test]
    fn test_user_allowed_specific() {
        let ch = WeComChannel::new("key".into(), vec!["user123".into()]);
        assert!(ch.is_user_allowed("user123"));
        assert!(!ch.is_user_allowed("other"));
    }

    #[test]
    fn test_user_denied_empty() {
        let ch = WeComChannel::new("key".into(), vec![]);
        assert!(!ch.is_user_allowed("anyone"));
    }

    #[test]
    fn test_config_serde_webhook() {
        let toml_str = r#"
webhook_key = "key-abc-123"
allowed_users = ["user1", "*"]
"#;
        let config: crate::config::schema::WeComConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.webhook_key.as_deref(), Some("key-abc-123"));
        assert!(!config.is_enterprise_app());
    }

    #[test]
    fn test_config_serde_enterprise() {
        let toml_str = r#"
corp_id = "ww1234567890"
corp_secret = "secret123"
agent_id = 1000002
token = "mytoken"
encoding_aes_key = "abcdefghijklmnopqrstuvwxyz0123456789ABCDEFG"
allowed_users = ["*"]
"#;
        let config: crate::config::schema::WeComConfig = toml::from_str(toml_str).unwrap();
        assert!(config.is_enterprise_app());
        assert_eq!(config.agent_id, Some(1000002));
    }

    #[test]
    fn test_crypto_signature() {
        let token = "testtoken";
        let timestamp = "1409735669";
        let nonce = "462889";
        let encrypt_msg = "test_encrypt_content";

        let sig = crypto::generate_signature(token, timestamp, nonce, encrypt_msg);
        assert!(crypto::verify_signature(token, timestamp, nonce, encrypt_msg, &sig));
        assert!(!crypto::verify_signature(token, timestamp, nonce, encrypt_msg, "bad"));
    }

    #[test]
    fn test_crypto_roundtrip() {
        let encoding_aes_key = "abcdefghijklmnopqrstuvwxyz0123456789ABCDEFG";
        let (key, iv) = crypto::decode_aes_key(encoding_aes_key).unwrap();
        let content = "Hello, WeCom Enterprise!";
        let corp_id = "ww1234567890";

        let encrypted = crypto::encrypt_message(&key, &iv, content, corp_id).unwrap();
        let (dec_content, dec_corp_id) = crypto::decrypt_message(&key, &iv, &encrypted).unwrap();
        assert_eq!(dec_content, content);
        assert_eq!(dec_corp_id, corp_id);
    }

    #[test]
    fn test_xml_extract_element() {
        let xml = r#"<xml><ToUserName><![CDATA[corp]]></ToUserName><Encrypt><![CDATA[enc_data]]></Encrypt></xml>"#;
        assert_eq!(xml::extract_element(xml, "ToUserName"), Some("corp".to_string()));
        assert_eq!(xml::extract_element(xml, "Encrypt"), Some("enc_data".to_string()));
        assert_eq!(xml::extract_element(xml, "Missing"), None);
    }

    #[test]
    fn test_xml_parse_message() {
        let xml = r#"<xml>
<ToUserName><![CDATA[ww1234567890]]></ToUserName>
<FromUserName><![CDATA[WuTianYang]]></FromUserName>
<CreateTime>1348831860</CreateTime>
<MsgType><![CDATA[text]]></MsgType>
<Content><![CDATA[Hello World]]></Content>
<MsgId>1234567890</MsgId>
</xml>"#;
        let msg = xml::parse_message_xml(xml).unwrap();
        assert_eq!(msg.from_user, "WuTianYang");
        assert_eq!(msg.msg_type, "text");
        assert_eq!(msg.content.as_deref(), Some("Hello World"));
    }
}
