//! Little-endian byte writer / bounds-checked reader.

use super::{Appearance, DecodeError, MAX_NICK_BYTES};

pub(super) struct Writer(pub(super) Vec<u8>);

impl Writer {
    pub(super) fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    pub(super) fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub(super) fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub(super) fn i32(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub(super) fn str8(&mut self, s: &str) {
        let b = truncate_utf8(s, MAX_NICK_BYTES).as_bytes();
        self.u8(b.len() as u8);
        self.0.extend_from_slice(b);
    }
    pub(super) fn str16(&mut self, s: &str, max: usize) {
        let b = truncate_utf8(s, max).as_bytes();
        self.u16(b.len() as u16);
        self.0.extend_from_slice(b);
    }
    pub(super) fn appearance(&mut self, a: &Appearance) {
        for v in [a.skin, a.hair_style, a.hair_color, a.shirt, a.pants] {
            self.u8(v);
        }
    }
}

pub(super) struct Reader<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub(super) fn new(b: &'a [u8]) -> Reader<'a> {
        Reader { b, pos: 0 }
    }

    /// Every byte consumed.
    pub(super) fn at_end(&self) -> bool {
        self.pos == self.b.len()
    }

    pub(super) fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        if self.pos + n > self.b.len() {
            return Err(DecodeError::TooShort);
        }
        let s = &self.b[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }
    pub(super) fn u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.take(1)?[0])
    }
    pub(super) fn array<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
        let mut a = [0; N];
        a.copy_from_slice(self.take(N)?);
        Ok(a)
    }
    pub(super) fn u16(&mut self) -> Result<u16, DecodeError> {
        Ok(u16::from_le_bytes(self.array()?))
    }
    pub(super) fn u32(&mut self) -> Result<u32, DecodeError> {
        Ok(u32::from_le_bytes(self.array()?))
    }
    pub(super) fn i32(&mut self) -> Result<i32, DecodeError> {
        Ok(i32::from_le_bytes(self.array()?))
    }
    pub(super) fn str8(&mut self) -> Result<String, DecodeError> {
        let n = self.u8()? as usize;
        if n > MAX_NICK_BYTES {
            return Err(DecodeError::Invalid("string too long"));
        }
        String::from_utf8(self.take(n)?.to_vec()).map_err(|_| DecodeError::Invalid("bad utf8"))
    }
    pub(super) fn appearance(&mut self) -> Result<Appearance, DecodeError> {
        Ok(Appearance { skin: self.u8()?, hair_style: self.u8()?, hair_color: self.u8()?, shirt: self.u8()?, pants: self.u8()? })
    }
    pub(super) fn str16(&mut self, max: usize) -> Result<String, DecodeError> {
        let n = self.u16()? as usize;
        if n > max {
            return Err(DecodeError::Invalid("string too long"));
        }
        String::from_utf8(self.take(n)?.to_vec()).map_err(|_| DecodeError::Invalid("bad utf8"))
    }
}

/// Cut a string to at most `max` bytes on a char boundary.
pub fn truncate_utf8(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}
