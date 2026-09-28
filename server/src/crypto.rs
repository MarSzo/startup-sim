//! Sealed game packets (stage 3 of accounts): with a key from logging in
//! (over HTTPS — it never crosses UDP), every packet of a session is
//! encrypted and signed:
//!
//! ```text
//! magic u16 | version u8 | type u8 (SEALED / SEALED_CONNECT)
//! | id: session token u32 (SEALED) or the login ticket, 32 B (SEALED_CONNECT)
//! | counter u64 | AES-256-CBC(packet, PKCS#7) | HMAC-SHA256(…)[..16]
//! ```
//!
//! Encrypt-then-MAC. The IV is the counter block encrypted with the key
//! (unpredictable); the MAC covers the direction byte, the header, the
//! counter and the ciphertext. The receiver drops anything it has seen
//! (a sliding window over the counters). Godot has AES and HMAC-SHA256
//! built in, so the client does the same natively.

use aes::cipher::{BlockDecryptMut, BlockEncrypt, BlockEncryptMut, KeyInit, KeyIvInit};
use aes::Aes256;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::protocol::{MAGIC, VERSION};

pub const SEALED: u8 = 0xF0;
pub const SEALED_CONNECT: u8 = 0xF1;
pub const TICKET_BYTES: usize = 32;
const MAC_BYTES: usize = 16;

/// Which way a packet goes (bound into the IV and the MAC).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    ToServer = 1,
    ToClient = 2,
}

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct Keys {
    enc: [u8; 32],
    mac: [u8; 32],
}

fn hmac(key: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let mut m = <HmacSha256 as Mac>::new_from_slice(key).expect("HMAC takes any key length");
    for p in parts {
        m.update(p);
    }
    m.finalize().into_bytes().into()
}

impl Keys {
    /// The two keys from the session key (from logging in).
    pub fn derive(session_key: &[u8; 32]) -> Keys {
        Keys { enc: hmac(session_key, &[b"startup-sim enc"]), mac: hmac(session_key, &[b"startup-sim mac"]) }
    }

    fn iv(&self, dir: Dir, counter: u64) -> [u8; 16] {
        let mut block = [0u8; 16];
        block[0] = dir as u8;
        block[1..9].copy_from_slice(&counter.to_le_bytes());
        let mut b = block.into();
        Aes256::new(&self.enc.into()).encrypt_block(&mut b);
        b.into()
    }

    /// `prefix` = magic, version, type and the id (token / ticket).
    pub fn seal(&self, dir: Dir, prefix: &[u8], counter: u64, inner: &[u8]) -> Vec<u8> {
        let ct = cbc::Encryptor::<Aes256>::new(&self.enc.into(), &self.iv(dir, counter).into())
            .encrypt_padded_vec_mut::<cbc::cipher::block_padding::Pkcs7>(inner);
        let mut out = Vec::with_capacity(prefix.len() + 8 + ct.len() + MAC_BYTES);
        out.extend_from_slice(prefix);
        out.extend_from_slice(&counter.to_le_bytes());
        out.extend_from_slice(&ct);
        let tag = hmac(&self.mac, &[&[dir as u8], &out]);
        out.extend_from_slice(&tag[..MAC_BYTES]);
        out
    }

    /// Check and decrypt; `prefix_len` = the header + id length. Returns
    /// (counter, the packet inside).
    pub fn open(&self, dir: Dir, prefix_len: usize, data: &[u8]) -> Option<(u64, Vec<u8>)> {
        if data.len() < prefix_len + 8 + 16 + MAC_BYTES {
            return None;
        }
        let (body, tag) = data.split_at(data.len() - MAC_BYTES);
        let want = hmac(&self.mac, &[&[dir as u8], body]);
        // Constant-time comparison.
        if tag.iter().zip(&want[..MAC_BYTES]).fold(0u8, |acc, (a, b)| acc | (a ^ b)) != 0 {
            return None;
        }
        let counter = u64::from_le_bytes(body[prefix_len..prefix_len + 8].try_into().ok()?);
        let ct = &body[prefix_len + 8..];
        let inner = cbc::Decryptor::<Aes256>::new(&self.enc.into(), &self.iv(dir, counter).into())
            .decrypt_padded_vec_mut::<cbc::cipher::block_padding::Pkcs7>(ct)
            .ok()?;
        Some((counter, inner))
    }
}

/// Header of a sealed session packet: magic, version, SEALED, token.
pub fn session_prefix(token: u32) -> [u8; 8] {
    let mut p = [0u8; 8];
    p[..2].copy_from_slice(&MAGIC.to_le_bytes());
    p[2] = VERSION;
    p[3] = SEALED;
    p[4..].copy_from_slice(&token.to_le_bytes());
    p
}

/// Header of a sealed Connect: magic, version, SEALED_CONNECT, ticket.
pub fn connect_prefix(ticket: &[u8; TICKET_BYTES]) -> Vec<u8> {
    let mut p = Vec::with_capacity(4 + TICKET_BYTES);
    p.extend_from_slice(&MAGIC.to_le_bytes());
    p.push(VERSION);
    p.push(SEALED_CONNECT);
    p.extend_from_slice(ticket);
    p
}

/// Counters already seen (the newest and the 64 before it).
#[derive(Debug, Default, Clone)]
pub struct ReplayWindow {
    top: u64,
    seen: u64,
}

impl ReplayWindow {
    /// True (and remembered) if `c` is new.
    pub fn accept(&mut self, c: u64) -> bool {
        if c == 0 {
            return false;
        }
        if c > self.top {
            let shift = c - self.top;
            self.seen = if shift >= 64 { 1 } else { (self.seen << shift) | 1 };
            self.top = c;
            return true;
        }
        let back = self.top - c;
        if back >= 64 || self.seen & (1 << back) != 0 {
            return false;
        }
        self.seen |= 1 << back;
        true
    }
}

/// A session's crypto state on one side.
pub struct Session {
    pub keys: Keys,
    pub send_counter: u64,
    pub window: ReplayWindow,
}

impl Session {
    pub fn new(keys: Keys) -> Session {
        Session { keys, send_counter: 0, window: ReplayWindow::default() }
    }
}

/// Hex to bytes (tickets, keys from the login API).
pub fn from_hex<const N: usize>(s: &str) -> Option<[u8; N]> {
    if s.len() != N * 2 {
        return None;
    }
    let mut out = [0u8; N];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(s.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_open_and_tamper() {
        let k = Keys::derive(&[7u8; 32]);
        let prefix = session_prefix(0x01020304);
        let sealed = k.seal(Dir::ToServer, &prefix, 5, b"hello, server");
        assert_eq!(k.open(Dir::ToServer, 8, &sealed), Some((5, b"hello, server".to_vec())));
        assert_eq!(k.open(Dir::ToClient, 8, &sealed), None, "the direction is part of the MAC");
        let mut bad = sealed.clone();
        bad[20] ^= 1;
        assert_eq!(k.open(Dir::ToServer, 8, &bad), None, "tampered");
        let other = Keys::derive(&[8u8; 32]);
        assert_eq!(other.open(Dir::ToServer, 8, &sealed), None, "another key");
        // Same plaintext, another counter: different bytes (fresh IV).
        assert_ne!(k.seal(Dir::ToServer, &prefix, 6, b"hello, server")[16..], sealed[16..]);
    }

    #[test]
    fn replay_window() {
        let mut w = ReplayWindow::default();
        assert!(w.accept(1) && w.accept(3) && w.accept(2));
        assert!(!w.accept(2) && !w.accept(3), "seen");
        assert!(w.accept(100));
        assert!(!w.accept(30), "too old");
        assert!(w.accept(99) && !w.accept(99));
        assert!(!w.accept(0));
    }

    #[test]
    fn hex() {
        assert_eq!(from_hex::<2>("0aff"), Some([0x0a, 0xff]));
        assert_eq!(from_hex::<2>("0af"), None);
        assert_eq!(from_hex::<2>("zzzz"), None);
    }
}
