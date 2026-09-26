## Binary UDP protocol - mirror of server/src/protocol.rs (see docs/PROTOCOL.md).
## Little-endian. Header: magic u16 | version u8 | type u8.
extends RefCounted

const MAGIC := 0x5354
const VERSION := 2
const MAX_PACKET := 1200
const MAX_NICK_BYTES := 16
const MAX_INPUTS_PER_PACKET := 8

const T_CONNECT := 1
const T_WELCOME := 2
const T_REJECT := 3
const T_INPUT := 4
const T_SNAPSHOT := 5
const T_PLAYER_INFO := 6
const T_INFO_REQUEST := 7
const T_PING := 8
const T_PONG := 9
const T_DISCONNECT := 10

const DISCONNECT_QUIT := 0
const DISCONNECT_TIMEOUT := 1
const DISCONNECT_KICKED := 2
const DISCONNECT_SHUTDOWN := 3
const DISCONNECT_SESSION_UNKNOWN := 4

const REJECT_REASONS := {1: "Serwer pełny", 2: "Niezgodna wersja protokołu", 3: "Nieprawidłowy nick"}
const DISCONNECT_REASONS := {0: "Rozłączono", 1: "Przekroczono czas", 2: "Wyrzucono", 3: "Serwer wyłączony", 4: "Sesja wygasła"}


static func _writer(type: int) -> StreamPeerBuffer:
	var b := StreamPeerBuffer.new()
	b.big_endian = false
	b.put_u16(MAGIC)
	b.put_u8(VERSION)
	b.put_u8(type)
	return b


## UTF-8 bytes of `s`, cut to `max_bytes` on a character boundary.
static func utf8_truncated(s: String, max_bytes: int) -> PackedByteArray:
	var out := PackedByteArray()
	for ch in s:
		var cb := ch.to_utf8_buffer()
		if out.size() + cb.size() > max_bytes:
			break
		out.append_array(cb)
	return out


static func encode_connect(nonce: int, nick: String) -> PackedByteArray:
	var b := _writer(T_CONNECT)
	b.put_u32(nonce)
	var nb := utf8_truncated(nick, MAX_NICK_BYTES)
	b.put_u8(nb.size())
	b.put_data(nb)
	return b.data_array


## `inputs`: consecutive input bytes, oldest first; the last has seq `last_seq`.
static func encode_input(token: int, ack_tick: int, last_seq: int, inputs: PackedByteArray) -> PackedByteArray:
	var b := _writer(T_INPUT)
	b.put_u32(token)
	b.put_u32(ack_tick)
	b.put_u32(last_seq)
	var n := mini(inputs.size(), MAX_INPUTS_PER_PACKET)
	b.put_u8(n)
	b.put_data(inputs.slice(inputs.size() - n))
	return b.data_array


static func encode_info_request(token: int, ids: Array) -> PackedByteArray:
	var b := _writer(T_INFO_REQUEST)
	b.put_u32(token)
	var n := mini(ids.size(), 255)
	b.put_u8(n)
	for i in n:
		b.put_u16(ids[i])
	return b.data_array


static func encode_ping(token: int, client_time: int) -> PackedByteArray:
	var b := _writer(T_PING)
	b.put_u32(token)
	b.put_u32(client_time & 0xFFFFFFFF)
	return b.data_array


static func encode_disconnect(token: int, reason: int) -> PackedByteArray:
	var b := _writer(T_DISCONNECT)
	b.put_u32(token)
	b.put_u8(reason)
	return b.data_array


## Bounds-checked reader over a packet.
class Reader:
	var b := StreamPeerBuffer.new()
	var ok := true

	func _init(bytes: PackedByteArray) -> void:
		b.big_endian = false
		b.data_array = bytes

	func _has(n: int) -> bool:
		if not ok or b.get_available_bytes() < n:
			ok = false
			return false
		return true

	func u8() -> int:
		return b.get_u8() if _has(1) else 0

	func u16() -> int:
		return b.get_u16() if _has(2) else 0

	func u32() -> int:
		return b.get_u32() if _has(4) else 0

	func i32() -> int:
		return b.get_32() if _has(4) else 0

	func str8() -> String:
		var n := u8()
		if n > MAX_NICK_BYTES or not _has(n):
			ok = false
			return ""
		var res: Array = b.get_data(n)
		return (res[1] as PackedByteArray).get_string_from_utf8()

	func at_end() -> bool:
		return b.get_available_bytes() == 0


## Decode a server->client packet. Returns {} for anything invalid.
static func decode(bytes: PackedByteArray) -> Dictionary:
	var r := Reader.new(bytes)
	if r.u16() != MAGIC or r.u8() != VERSION:
		return {}
	var t := r.u8()
	var p := {"type": t}
	match t:
		T_WELCOME:
			p.nonce = r.u32()
			p.player_id = r.u16()
			p.token = r.u32()
			p.tick_hz = r.u8()
			p.input_hz = r.u8()
			p.map_crc = r.u32()
			p.server_tick = r.u32()
		T_REJECT:
			p.reason = r.u8()
		T_SNAPSHOT:
			p.tick = r.u32()
			p.last_input_seq = r.u32()
			p.frag_idx = r.u8()
			p.frag_cnt = r.u8()
			p.self_x = r.i32()
			p.self_y = r.i32()
			p.floor = r.u8()
			p.room = r.u16()
			p.self_lock = r.u8()
			p.self_prev_input = r.u8()
			var n := r.u8()
			var ents := []
			for i in n:
				ents.append({"id": r.u16(), "kind": r.u8(), "x": r.i32(), "y": r.i32(), "flags": r.u8()})
			p.entities = ents
		T_PLAYER_INFO:
			var n := r.u8()
			var players := []
			for i in n:
				players.append({"id": r.u16(), "nick": r.str8()})
			p.players = players
		T_PONG:
			p.client_time = r.u32()
			p.server_tick = r.u32()
		T_DISCONNECT:
			p.token = r.u32()
			p.reason = r.u8()
		_:
			return {}
	if not r.ok or not r.at_end():
		return {}
	return p
