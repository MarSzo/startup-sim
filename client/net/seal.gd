## Sealed game packets (the same as server/src/crypto.rs): AES-256-CBC +
## HMAC-SHA256 (encrypt-then-MAC) with the session key from logging in.
##   magic u16 | version u8 | SEALED / SEALED_CONNECT | token u32 / ticket 32 B
##   | counter u64 | ciphertext | MAC[16]
## The IV is the counter block encrypted with the key; the MAC covers the
## direction, the header, the counter and the ciphertext.
extends RefCounted

const SEALED := 0xF0
const SEALED_CONNECT := 0xF1
const TO_SERVER := 1
const TO_CLIENT := 2
const MAC_BYTES := 16

var enc := PackedByteArray()
var mac := PackedByteArray()
# Counters of packets from the server already seen (sliding window).
var _top := 0
var _seen := 0


static func from_key(key: PackedByteArray):
	var s = load("res://net/seal.gd").new()
	s.enc = hmac(key, "startup-sim enc".to_utf8_buffer())
	s.mac = hmac(key, "startup-sim mac".to_utf8_buffer())
	return s


static func hmac(key: PackedByteArray, msg: PackedByteArray) -> PackedByteArray:
	return Crypto.new().hmac_digest(HashingContext.HASH_SHA256, key, msg)


static func _u64(v: int) -> PackedByteArray:
	var b := PackedByteArray()
	b.resize(8)
	b.encode_s64(0, v)
	return b


func _iv(dir: int, counter: int) -> PackedByteArray:
	var block := PackedByteArray()
	block.resize(16)
	block[0] = dir
	block.encode_s64(1, counter)
	var aes := AESContext.new()
	aes.start(AESContext.MODE_ECB_ENCRYPT, enc)
	var iv := aes.update(block)
	aes.finish()
	return iv


static func session_prefix(magic: int, version: int, token: int) -> PackedByteArray:
	var p := PackedByteArray()
	p.resize(8)
	p.encode_u16(0, magic)
	p[2] = version
	p[3] = SEALED
	p.encode_u32(4, token)
	return p


static func connect_prefix(magic: int, version: int, ticket: PackedByteArray) -> PackedByteArray:
	var p := PackedByteArray()
	p.resize(4)
	p.encode_u16(0, magic)
	p[2] = version
	p[3] = SEALED_CONNECT
	p.append_array(ticket)
	return p


func seal(dir: int, prefix: PackedByteArray, counter: int, inner: PackedByteArray) -> PackedByteArray:
	var pad := 16 - inner.size() % 16
	var padded := inner.duplicate()
	for i in pad:
		padded.append(pad)
	var aes := AESContext.new()
	aes.start(AESContext.MODE_CBC_ENCRYPT, enc, _iv(dir, counter))
	var ct := aes.update(padded)
	aes.finish()
	var out := prefix.duplicate()
	out.append_array(_u64(counter))
	out.append_array(ct)
	var signed := PackedByteArray([dir])
	signed.append_array(out)
	out.append_array(hmac(mac, signed).slice(0, MAC_BYTES))
	return out


## [counter, inner packet], or [] if it doesn't check out.
func open(dir: int, prefix_len: int, data: PackedByteArray) -> Array:
	if data.size() < prefix_len + 8 + 16 + MAC_BYTES:
		return []
	var body := data.slice(0, data.size() - MAC_BYTES)
	var tag := data.slice(data.size() - MAC_BYTES)
	var signed := PackedByteArray([dir])
	signed.append_array(body)
	var want := hmac(mac, signed)
	var diff := 0
	for i in MAC_BYTES:
		diff |= tag[i] ^ want[i]
	if diff != 0:
		return []
	var counter := body.decode_s64(prefix_len)
	var ct := body.slice(prefix_len + 8)
	if ct.size() == 0 or ct.size() % 16 != 0:
		return []
	var aes := AESContext.new()
	aes.start(AESContext.MODE_CBC_DECRYPT, enc, _iv(dir, counter))
	var plain := aes.update(ct)
	aes.finish()
	var pad := plain[plain.size() - 1]
	if pad < 1 or pad > 16:
		return []
	return [counter, plain.slice(0, plain.size() - pad)]


## A counter from the server we haven't seen yet (then remembered).
func accept(c: int) -> bool:
	if c <= 0:
		return false
	if c > _top:
		var shift := c - _top
		_seen = 1 if shift >= 64 else ((_seen << shift) | 1)
		_top = c
		return true
	var back := _top - c
	if back >= 64 or (_seen >> back) & 1 == 1:
		return false
	_seen |= 1 << back
	return true


## A new session (after a Welcome): the server counts from 1 again.
func reset_window() -> void:
	_top = 0
	_seen = 0
