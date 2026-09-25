## UDP connection to the game server: handshake, keepalive/ping, timeouts.
## Emits decoded packets; the game layer decides what to do with them.
extends Node

const Protocol = preload("res://net/protocol.gd")

signal connected(welcome: Dictionary)
signal disconnected(reason: String)
signal packet_received(p: Dictionary)

enum State { IDLE, CONNECTING, CONNECTED }

const CONNECT_RETRY_SEC := 0.5
const CONNECT_TIMEOUT_SEC := 5.0
const SERVER_TIMEOUT_SEC := 5.0
const PING_INTERVAL_SEC := 1.0

var state := State.IDLE
var udp := PacketPeerUDP.new()
var nick := ""
var nonce := 0
var player_id := 0
var token := 0
var rtt_ms := 0.0

var _connect_elapsed := 0.0
var _retry_timer := 0.0
var _since_heard := 0.0
var _ping_timer := 0.0

# Traffic stats (bytes per second, updated once a second).
var bytes_in_per_sec := 0
var bytes_out_per_sec := 0
var _bytes_in := 0
var _bytes_out := 0
var _stats_timer := 0.0


func connect_to_server(address: String, p_nick: String) -> String:
	var host := address
	var port := 7777
	var colon := address.rfind(":")
	if colon > 0:
		host = address.substr(0, colon)
		port = int(address.substr(colon + 1))
	if not host.is_valid_ip_address():
		host = IP.resolve_hostname(host, IP.TYPE_IPV4)
		if host == "":
			return "Nie można rozwiązać adresu"
	udp.close()
	var err := udp.connect_to_host(host, port)
	if err != OK:
		return "Błąd gniazda UDP (%d)" % err
	nick = p_nick
	nonce = randi()
	state = State.CONNECTING
	_connect_elapsed = 0.0
	_retry_timer = 0.0
	return ""


func send(bytes: PackedByteArray) -> void:
	if state == State.IDLE:
		return
	_bytes_out += bytes.size()
	udp.put_packet(bytes)


func close(reason: String = "") -> void:
	if state == State.CONNECTED:
		send(Protocol.encode_disconnect(token, 0))
	var was := state
	state = State.IDLE
	udp.close()
	if was != State.IDLE and reason != "":
		disconnected.emit(reason)


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
	if state == State.CONNECTING:
		_connect_elapsed += delta
		_retry_timer -= delta
		if _connect_elapsed > CONNECT_TIMEOUT_SEC:
			close("Serwer nie odpowiada")
		elif _retry_timer <= 0.0:
			send(Protocol.encode_connect(nonce, nick))
			_retry_timer = CONNECT_RETRY_SEC
	elif state == State.CONNECTED:
		_since_heard += delta
		if _since_heard > SERVER_TIMEOUT_SEC:
			close("Utracono połączenie z serwerem")
			return
		_ping_timer -= delta
		if _ping_timer <= 0.0:
			send(Protocol.encode_ping(token, Time.get_ticks_msec()))
			_ping_timer = PING_INTERVAL_SEC


func _poll() -> void:
	while state != State.IDLE and udp.get_available_packet_count() > 0:
		var bytes := udp.get_packet()
		_bytes_in += bytes.size()
		var p := Protocol.decode(bytes)
		if p.is_empty():
			continue
		_since_heard = 0.0
		match p.type:
			Protocol.T_WELCOME:
				if state == State.CONNECTING and p.nonce == nonce:
					player_id = p.player_id
					token = p.token
					state = State.CONNECTED
					_ping_timer = 0.0
					connected.emit(p)
			Protocol.T_REJECT:
				close("Odrzucono: " + Protocol.REJECT_REASONS.get(p.reason, str(p.reason)))
			Protocol.T_DISCONNECT:
				if p.token == token:
					udp.close()
					state = State.IDLE
					disconnected.emit(Protocol.DISCONNECT_REASONS.get(p.reason, "Rozłączono"))
			Protocol.T_PONG:
				var rtt := float((Time.get_ticks_msec() - p.client_time) & 0xFFFFFFFF)
				rtt_ms = rtt if rtt_ms == 0.0 else lerpf(rtt_ms, rtt, 0.3)
				packet_received.emit(p)
			_:
				if state == State.CONNECTED:
					packet_received.emit(p)
