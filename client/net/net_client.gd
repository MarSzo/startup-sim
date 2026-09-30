## UDP connection to the game server: handshake, keepalive/ping, timeouts,
## surviving network changes and automatic reconnects.
## Emits decoded packets; the game layer decides what to do with them.
##
## The server identifies us by session token, not by address, so when the
## network changes (Wi-Fi <-> LTE, laptop wakes up) we just open a new socket
## and keep sending with the same token. If the session is gone (server
## timeout, restart) we reconnect with a fresh Connect, for up to
## RECONNECT_WINDOW_SEC, before giving up to the start screen.
extends Node

const Seal = preload("res://net/seal.gd")
const Protocol = preload("res://net/protocol.gd")
const Departments = preload("res://net/departments.gd")

signal connected(welcome: Dictionary)
## Lost the session; trying to get a new one (game should freeze, not quit).
signal reconnecting(reason: String)
signal disconnected(reason: String)
## The server refused the Connect (Protocol.REJECT_*), just before `disconnected`.
signal rejected(reason: int)
signal packet_received(p: Dictionary)

enum State { IDLE, CONNECTING, CONNECTED }

const DEFAULT_PORT := 7777
const CONNECT_RETRY_SEC := 0.5
const CONNECT_TIMEOUT_SEC := 5.0
const SERVER_TIMEOUT_SEC := 5.0
## Silence after which we suspect a network change and re-open the socket.
const REBIND_AFTER_SEC := 1.5
const RECONNECT_WINDOW_SEC := 30.0
const PING_INTERVAL_SEC := 1.0

var state := State.IDLE
var udp := PacketPeerUDP.new()
var nick := ""
var profile := {}
## Login ticket (HTTPS); "" = playing as a guest.
var ticket := ""
## With an account every packet is sealed with the session key
## (net/seal.gd); null = a guest, plain packets.
var _seal = null
var _ticket_raw := PackedByteArray()
var _send_counter := 0
var nonce := 0
var player_id := 0
var token := 0
var rtt_ms := 0.0
var server_ip := ""
var rebinds := 0
var reconnects := 0

var _host := ""
var _port := DEFAULT_PORT
var _reconnecting := false
var _reconnect_elapsed := 0.0
var _connect_elapsed := 0.0
var _retry_timer := 0.0
var _since_heard := 0.0
var _rebind_timer := 0.0
var _ping_timer := 0.0

# Traffic stats (bytes per second, updated once a second).
var bytes_in_per_sec := 0
var bytes_out_per_sec := 0
var _bytes_in := 0
var _bytes_out := 0
var _stats_timer := 0.0


## "host", "host:port", "1.2.3.4:port", "[v6]:port", "[v6]" or bare "v6".
## Returns [host, port], or [] if malformed.
static func parse_address(address: String) -> Array:
	var a := address.strip_edges()
	if a.is_empty():
		return []
	var host := a
	var port := DEFAULT_PORT
	if a.begins_with("["):
		var close_i := a.find("]")
		if close_i < 0:
			return []
		host = a.substr(1, close_i - 1)
		var rest := a.substr(close_i + 1)
		if rest.begins_with(":"):
			if not rest.substr(1).is_valid_int():
				return []
			port = int(rest.substr(1))
		elif rest != "":
			return []
	elif a.count(":") == 1:
		var i := a.find(":")
		host = a.substr(0, i)
		if not a.substr(i + 1).is_valid_int():
			return []
		port = int(a.substr(i + 1))
	# else: hostname / IPv4 without port, or bare IPv6 (2+ colons) without port
	if host.is_empty() or port < 1 or port > 65535:
		return []
	return [host, port]


func connect_to_server(address: String, p_nick: String, p_profile: Dictionary, p_ticket := "", key_hex := "") -> String:
	var hp := parse_address(address)
	if hp.is_empty():
		return "Nieprawidłowy adres serwera"
	_host = hp[0]
	_port = hp[1]
	nick = p_nick
	profile = p_profile
	ticket = p_ticket
	_seal = Seal.from_key(key_hex.hex_decode()) if key_hex.length() == 64 else null
	_ticket_raw = p_ticket.hex_decode() if _seal else PackedByteArray()
	_reconnecting = false
	reconnects = 0
	return _start_connect()


func _resolve() -> String:
	if _host.is_valid_ip_address():
		return _host
	# TYPE_ANY: works on IPv6-only networks (App Store requirement) and IPv4.
	return IP.resolve_hostname(_host, IP.TYPE_ANY)


func _open_socket() -> Error:
	udp.close()
	return udp.connect_to_host(server_ip, _port)


func _start_connect() -> String:
	server_ip = _resolve()
	if server_ip == "":
		return "Nie można rozwiązać adresu"
	var err := _open_socket()
	if err != OK:
		return "Błąd gniazda UDP (%d)" % err
	nonce = randi()
	token = 0
	if _seal:
		_seal.reset_window()  # a new session counts from 1
	state = State.CONNECTING
	_connect_elapsed = 0.0
	_retry_timer = 0.0
	return ""


func send(bytes: PackedByteArray) -> void:
	if state == State.IDLE:
		return
	if _seal:
		# The counter only goes up (the server refuses a replayed Connect).
		_send_counter += 1
		var prefix: PackedByteArray
		if bytes[3] == Protocol.T_CONNECT:
			prefix = Seal.connect_prefix(Protocol.MAGIC, Protocol.VERSION, _ticket_raw)
		else:
			prefix = Seal.session_prefix(Protocol.MAGIC, Protocol.VERSION, token)
		bytes = _seal.seal(Seal.TO_SERVER, prefix, _send_counter, bytes)
	_bytes_out += bytes.size()
	udp.put_packet(bytes)


func is_playing() -> bool:
	return state == State.CONNECTED


func close(reason: String = "") -> void:
	if state == State.CONNECTED:
		send(Protocol.encode_disconnect(token, 0))
	var was := state
	state = State.IDLE
	_reconnecting = false
	udp.close()
	if was != State.IDLE and reason != "":
		disconnected.emit(reason)


## Session lost: get a new one without leaving the game (or give up).
func _begin_reconnect(reason: String) -> void:
	if not _reconnecting:
		_reconnecting = true
		_reconnect_elapsed = 0.0
		reconnecting.emit(reason)
	reconnects += 1
	var err := _start_connect()
	if err != "":
		# e.g. DNS fails while offline: stay in CONNECTING-like limbo and retry.
		state = State.CONNECTING
		_connect_elapsed = 0.0


## Same session, new socket: new source port / interface after a network change.
func _rebind() -> void:
	var ip := _resolve()
	if ip != "":
		server_ip = ip
	if _open_socket() == OK:
		rebinds += 1
		send(Protocol.encode_ping(token, Time.get_ticks_msec()))


func _notification(what: int) -> void:
	# Mobile: back from background - the network may have changed meanwhile.
	if what == NOTIFICATION_APPLICATION_RESUMED and state == State.CONNECTED:
		_rebind()


func _process(delta: float) -> void:
	if state == State.IDLE:
		return
	_poll()
	if state == State.IDLE:
		return
	_stats_timer += delta
	if _stats_timer >= 1.0:
		bytes_in_per_sec = int(_bytes_in / _stats_timer)
		bytes_out_per_sec = int(_bytes_out / _stats_timer)
		_bytes_in = 0
		_bytes_out = 0
		_stats_timer = 0.0
	if _reconnecting:
		_reconnect_elapsed += delta
		if _reconnect_elapsed > RECONNECT_WINDOW_SEC:
			close("Utracono połączenie z serwerem")
			return
	if state == State.CONNECTING:
		_connect_elapsed += delta
		_retry_timer -= delta
		if _connect_elapsed > CONNECT_TIMEOUT_SEC:
			if _reconnecting:
				_begin_reconnect("")  # new attempt, maybe DNS/route is back
			else:
				close("Serwer nie odpowiada")
		elif _retry_timer <= 0.0:
			if server_ip == "":
				server_ip = _resolve()
				if server_ip != "":
					_open_socket()
			if server_ip != "":
				send(Protocol.encode_connect(nonce, nick, profile, ticket))
			_retry_timer = CONNECT_RETRY_SEC
	elif state == State.CONNECTED:
		_since_heard += delta
		if _since_heard > SERVER_TIMEOUT_SEC:
			_begin_reconnect("Brak odpowiedzi serwera")
			return
		if _since_heard > REBIND_AFTER_SEC:
			_rebind_timer -= delta
			if _rebind_timer <= 0.0:
				_rebind()
				_rebind_timer = REBIND_AFTER_SEC
		_ping_timer -= delta
		if _ping_timer <= 0.0:
			send(Protocol.encode_ping(token, Time.get_ticks_msec()))
			_ping_timer = PING_INTERVAL_SEC


func _poll() -> void:
	while state != State.IDLE and udp.get_available_packet_count() > 0:
		var bytes := udp.get_packet()
		_bytes_in += bytes.size()
		if _seal and bytes.size() > 3:
			if bytes[3] == Seal.SEALED:
				var opened: Array = _seal.open(Seal.TO_CLIENT, 8, bytes)
				if opened.is_empty() or not _seal.accept(opened[0]):
					continue  # forged, damaged or replayed
				bytes = opened[1]
			elif bytes[3] != Protocol.T_REJECT and bytes[3] != Protocol.T_DISCONNECT:
				continue  # a logged-in session only takes sealed packets
		var p := Protocol.decode(bytes)
		if p.is_empty():
			continue
		match p.type:
			Protocol.T_WELCOME:
				if state == State.CONNECTING and p.nonce == nonce:
					player_id = p.player_id
					token = p.token
					state = State.CONNECTED
					_reconnecting = false
					_since_heard = 0.0
					_rebind_timer = 0.0
					_ping_timer = 0.0
					connected.emit(p)
			Protocol.T_REJECT:
				if state == State.CONNECTING:
					rejected.emit(p.reason)
					close(Protocol.REJECT_REASONS.get(p.reason, "Odrzucono (%d)" % p.reason))
			Protocol.T_DISCONNECT:
				if state == State.CONNECTED and p.token == token:
					if p.reason == Protocol.DISCONNECT_TIMEOUT or p.reason == Protocol.DISCONNECT_SESSION_UNKNOWN:
						_begin_reconnect(Protocol.DISCONNECT_REASONS.get(p.reason, "Rozłączono"))
					else:
						udp.close()
						state = State.IDLE
						disconnected.emit(Protocol.DISCONNECT_REASONS.get(p.reason, "Rozłączono"))
			Protocol.T_PONG:
				if state == State.CONNECTED:
					_since_heard = 0.0
					var rtt := float((Time.get_ticks_msec() - p.client_time) & 0xFFFFFFFF)
					rtt_ms = rtt if rtt_ms == 0.0 else lerpf(rtt_ms, rtt, 0.3)
					packet_received.emit(p)
			_:
				if state == State.CONNECTED:
					_since_heard = 0.0
					if p.type == Protocol.T_DEPARTMENTS:
						Departments.set_list(p.list)
					packet_received.emit(p)
