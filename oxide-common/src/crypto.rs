use anyhow::{Context, Result};
use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Key, Nonce,
};
use hkdf::Hkdf;
use rand::RngCore;
use sha2::Sha256;
use x25519_dalek::{EphemeralSecret, PublicKey, StaticSecret};

// ── XOR codec (compile-time config obfuscation) ───────────────────────────────

pub fn xor_encode(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, &b)| b ^ key[i % key.len()])
        .collect()
}

pub fn xor_decode(data: &[u8], key: &[u8]) -> Vec<u8> {
    xor_encode(data, key) // XOR is symmetric
}

// ── x25519 key generation ─────────────────────────────────────────────────────

pub fn generate_x25519_keypair() -> (StaticSecret, PublicKey) {
    let secret = StaticSecret::random_from_rng(OsRng);
    let public = PublicKey::from(&secret);
    (secret, public)
}

pub fn pubkey_to_hex(pk: &PublicKey) -> String {
    hex::encode(pk.as_bytes())
}

pub fn pubkey_from_hex(s: &str) -> Result<PublicKey> {
    let bytes = hex::decode(s).context("bad hex")?;
    let arr: [u8; 32] = bytes.try_into().map_err(|_| anyhow::anyhow!("wrong length"))?;
    Ok(PublicKey::from(arr))
}

pub fn secret_from_hex(s: &str) -> Result<StaticSecret> {
    let bytes = hex::decode(s).context("bad hex")?;
    let arr: [u8; 32] = bytes.try_into().map_err(|_| anyhow::anyhow!("wrong length"))?;
    Ok(StaticSecret::from(arr))
}

// ── ECDH handshake → ChaCha20-Poly1305 session key ───────────────────────────

/// Server: generate ephemeral keypair, derive session key from agent's pubkey.
pub struct ServerHandshake {
    pub ephemeral_pub: PublicKey,
    pub session_key:   [u8; 32],
}

impl ServerHandshake {
    pub fn new(agent_ephemeral_pub: &PublicKey, server_static_secret: &StaticSecret) -> Self {
        let shared = server_static_secret.diffie_hellman(agent_ephemeral_pub);
        let hk = Hkdf::<Sha256>::new(None, shared.as_bytes());
        let mut session_key = [0u8; 32];
        hk.expand(b"oxide-session-key", &mut session_key)
            .expect("hkdf expand");
        ServerHandshake {
            ephemeral_pub: PublicKey::from(server_static_secret),
            session_key,
        }
    }
}

/// Agent: use own ephemeral secret + server's static pubkey to derive session key.
pub struct AgentHandshake {
    pub ephemeral_pub: PublicKey,
    pub session_key:   [u8; 32],
}

impl AgentHandshake {
    pub fn new(server_static_pub: &PublicKey) -> Self {
        let ephemeral = EphemeralSecret::random_from_rng(OsRng);
        let ephemeral_pub = PublicKey::from(&ephemeral);
        let shared = ephemeral.diffie_hellman(server_static_pub);
        let hk = Hkdf::<Sha256>::new(None, shared.as_bytes());
        let mut session_key = [0u8; 32];
        hk.expand(b"oxide-session-key", &mut session_key)
            .expect("hkdf expand");
        AgentHandshake { ephemeral_pub, session_key }
    }
}

// ── ChaCha20-Poly1305 symmetric crypto ────────────────────────────────────────

pub struct SessionCipher {
    cipher: ChaCha20Poly1305,
}

impl SessionCipher {
    pub fn new(key: &[u8; 32]) -> Self {
        let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
        SessionCipher { cipher }
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ct = self.cipher.encrypt(nonce, plaintext)
            .map_err(|e| anyhow::anyhow!("encrypt: {e}"))?;
        // Prepend nonce so receiver can decrypt
        let mut out = Vec::with_capacity(12 + ct.len());
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ct);
        Ok(out)
    }

    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 12 {
            anyhow::bail!("ciphertext too short");
        }
        let (nonce_bytes, ct) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        self.cipher.decrypt(nonce, ct)
            .map_err(|e| anyhow::anyhow!("decrypt: {e}"))
    }
}
