## Accounts: the login API over HTTPS next to the game port (port + 1).
## The server's own certificate is pinned on first contact (like SSH): its
## PEM comes from /api/cert and is kept in user://known_servers.cfg; after
## that every request checks it. A server with a real certificate (no
## /api/cert) is checked the usual way. "Remember me" keeps only the
## refresh token (never the password) in user://auth.cfg.
extends Node

const NetClient = preload("res://net/net_client.gd")

const PINS := "user://known_servers.cfg"
## The game's server (the default in the login screen).
const DEFAULT_SERVER := "178.105.233.184:7777"
const SESSION := "user://auth.cfg"
## The name in the server's own certificate.
const SELF_SIGNED_NAME := "startup-sim"

var _http := HTTPRequest.new()
var _busy := false


func _ready() -> void:
	_http.timeout = 10.0
	add_child(_http)


## "host:port" of the game -> the API base URL.
static func api_base(address: String) -> String:
	var hp := NetClient.parse_address(address)
	if hp.is_empty():
		return ""
	var host: String = hp[0]
	if host.contains(":"):
		host = "[%s]" % host
	return "https://%s:%d/api/" % [host, int(hp[1]) + 1]


# ------------------------------------------------------------------ calls

func register(address: String, nick: String, password: String) -> Dictionary:
	return await _post(address, "register", {"nick": nick, "password": password})


func login(address: String, nick: String, password: String) -> Dictionary:
	return await _post(address, "login", {"nick": nick, "password": password})


func change_password(address: String, nick: String, password: String, new_password: String) -> Dictionary:
	return await _post(address, "password", {"nick": nick, "password": password, "new_password": new_password})


func refresh(address: String, token: String) -> Dictionary:
	return await _post(address, "refresh", {"refresh": token})


func logout(address: String, token: String) -> void:
	await _post(address, "logout", {"refresh": token})


## POST JSON; always answers {ok, error, ...} (never throws).
func _post(address: String, path: String, body: Dictionary) -> Dictionary:
	var base := api_base(address)
	if base == "":
		return {"ok": false, "error": "Nieprawidłowy adres serwera"}
	while _busy:
		await get_tree().process_frame
	_busy = true
	var tls := await _tls_for(address, base)
	if tls == null:
		_busy = false
		return {"ok": false, "error": "Brak połączenia z serwerem logowania (%s)" % base.trim_suffix("api/")}
	_http.set_tls_options(tls)
	var err := _http.request(base + path, ["Content-Type: application/json"], HTTPClient.METHOD_POST, JSON.stringify(body))
	if err != OK:
		_busy = false
		return {"ok": false, "error": "Błąd zapytania (%d)" % err}
	var res: Array = await _http.request_completed
	_busy = false
	if res[0] == HTTPRequest.RESULT_TLS_HANDSHAKE_ERROR:
		return {"ok": false, "cert_changed": true,
			"error": "Certyfikat serwera się zmienił! Jeśli to na pewno Twój serwer (np. po reinstalacji), zaufaj nowemu."}
	if res[0] != HTTPRequest.RESULT_SUCCESS:
		return {"ok": false, "error": "Brak połączenia z serwerem logowania"}
	if res[1] == 204:
		return {"ok": true}
	var parsed = JSON.parse_string((res[3] as PackedByteArray).get_string_from_utf8())
	if typeof(parsed) != TYPE_DICTIONARY:
		return {"ok": false, "error": "Dziwna odpowiedź serwera (%d)" % res[1]}
	return parsed


## TLS for this server: its pinned certificate, the one it offers on first
## contact, or the usual checks for a real certificate. null = unreachable.
func _tls_for(address: String, base: String) -> TLSOptions:
	var pem := pinned(address)
	if pem == "":
		# First contact: take the certificate it offers (and remember it).
		_http.set_tls_options(TLSOptions.client_unsafe())
		if _http.request(base + "cert") != OK:
			return null
		var res: Array = await _http.request_completed
		if res[0] != HTTPRequest.RESULT_SUCCESS:
			return null
		if res[1] == 404:
			return TLSOptions.client()  # a real certificate
		pem = (res[3] as PackedByteArray).get_string_from_utf8()
		if not pem.begins_with("-----BEGIN CERTIFICATE-----"):
			return null
		_pin(address, pem)
	var cert := X509Certificate.new()
	if cert.load_from_string(pem) != OK:
		forget_pin(address)
		return null
	return TLSOptions.client(cert, SELF_SIGNED_NAME)


# ------------------------------------------------------------- storage

## The certificate we trust for `address`: shipped with the game
## (res://net/pins/<host>_<port>.pem, from deploy/pull-cert.sh) or pinned on
## first contact.
static func pinned(address: String) -> String:
	var shipped := "res://net/pins/%s.pem" % address.replace(":", "_").replace("[", "").replace("]", "")
	if FileAccess.file_exists(shipped):
		return FileAccess.get_file_as_string(shipped)
	var cfg := ConfigFile.new()
	if cfg.load(PINS) != OK:
		return ""
	return str(cfg.get_value("pins", address, ""))


static func _pin(address: String, pem: String) -> void:
	var cfg := ConfigFile.new()
	cfg.load(PINS)
	cfg.set_value("pins", address, pem)
	cfg.save(PINS)


static func forget_pin(address: String) -> void:
	var cfg := ConfigFile.new()
	if cfg.load(PINS) == OK and cfg.has_section_key("pins", address):
		cfg.erase_section_key("pins", address)
		cfg.save(PINS)


## The remembered login: {address, nick, refresh} or {}.
static func remembered() -> Dictionary:
	var cfg := ConfigFile.new()
	if cfg.load(SESSION) != OK or str(cfg.get_value("session", "refresh", "")) == "":
		return {}
	return {"address": str(cfg.get_value("session", "address", "")), "nick": str(cfg.get_value("session", "nick", "")),
		"refresh": str(cfg.get_value("session", "refresh", ""))}


static func remember(address: String, nick: String, token: String) -> void:
	var cfg := ConfigFile.new()
	cfg.set_value("session", "address", address)
	cfg.set_value("session", "nick", nick)
	cfg.set_value("session", "refresh", token)
	cfg.save(SESSION)


static func forget() -> void:
	var cfg := ConfigFile.new()
	cfg.set_value("session", "refresh", "")
	cfg.save(SESSION)
