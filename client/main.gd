## Entry point: character creation <-> game. User args (after `--`):
##   --nick=Ala --server=127.0.0.1:7777 --autoconnect --debug --autowalk
##   --screenshot=/path.png [--screenshot-delay=5]  (dev: save a frame and quit;
##     several delays "5,12,20" save path_1.png, path_2.png, ... and quit after the last)
##   --auto-recruit=1 [--auto-recruit-delay=2]  (dev: apply for offer 1, answer
##     at random until hired, waiting N s before each click)
extends Node

const NetClient = preload("res://net/net_client.gd")
const Building = preload("res://map/building.gd")
const Game = preload("res://game/game.gd")
const CharacterScreen = preload("res://ui/character_screen.gd")
const Desktop = preload("res://ui/desktop.gd")
const Protocol = preload("res://net/protocol.gd")

const BUILDING_PATH := "res://maps/building.json"

var args := {}
var net := NetClient.new()
var building
var start := CharacterScreen.new()
var profile := {}
var game: Node = null
var portal_layer := CanvasLayer.new()
var portal := Desktop.new()


func _ready() -> void:
	for a in OS.get_cmdline_user_args():
		var kv: PackedStringArray = a.trim_prefix("--").split("=", true, 1)
		args[kv[0]] = kv[1] if kv.size() > 1 else ""
	get_tree().auto_accept_quit = false
	building = Building.new()
	building.load_path(BUILDING_PATH)
	add_child(net)
	net.connected.connect(_on_connected)
	net.disconnected.connect(_on_disconnected)
	net.reconnecting.connect(_on_reconnecting)
	net.packet_received.connect(_on_packet)
	portal_layer.layer = 20
	portal_layer.visible = false
	add_child(portal_layer)
	portal_layer.add_child(portal)
	portal.auto_offer = int(args.get("auto-recruit", "0"))
	portal.auto_delay = float(args.get("auto-recruit-delay", "0"))
	portal.apply.connect(func(offer, motivation): net.send(Protocol.encode_apply(net.token, offer, motivation)))
	portal.answer.connect(func(a, i, c): net.send(Protocol.encode_answer(net.token, a, i, c)))
	portal.portal_action.connect(func(action, arg): net.send(Protocol.encode_portal_action(net.token, action, arg)))
	var ui := CanvasLayer.new()
	add_child(ui)
	ui.add_child(start)
	start.connect_pressed.connect(_on_connect_pressed)
	if args.has("nick") or args.has("autoconnect"):
		start.set_defaults(args.get("nick", "Gracz%d" % randi_range(100, 999)), args.get("server", ""))
	elif args.has("server"):
		start.addr_edit.text = args["server"]
	if building.error != "":
		start.set_status("Błąd mapy: " + building.error, true)
		start.set_busy(true)
	elif args.has("autoconnect"):
		# Dev: connect with the filled-in character without saving it.
		var err: String = start.validation_error()
		if err == "":
			_on_connect_pressed(start.nick_edit.text.strip_edges(), start.profile(), start.addr_edit.text)
		else:
			start.set_status(err, true)
	if args.has("screenshot"):
		_take_screenshots(args["screenshot"], args.get("screenshot-delay", "5").split(","))


func _take_screenshots(path: String, delays: PackedStringArray) -> void:
	var elapsed := 0.0
	for i in delays.size():
		var at := float(delays[i])
		await get_tree().create_timer(maxf(at - elapsed, 0.0)).timeout
		elapsed = at
		await RenderingServer.frame_post_draw
		var out := path if delays.size() == 1 else "%s_%d.png" % [path.get_basename(), i + 1]
		get_viewport().get_texture().get_image().save_png(out)
		print("screenshot saved: ", out)
	if game:
		print(game.debug_text())
	net.close()
	get_tree().quit()


func _on_connect_pressed(nick: String, p_profile: Dictionary, address: String) -> void:
	profile = p_profile
	var err: String = net.connect_to_server(address, nick, p_profile)
	if err != "":
		start.set_status(err, true)
		return
	start.set_status("Łączenie z %s..." % address)
	start.set_busy(true)


func _on_connected(welcome: Dictionary) -> void:
	if welcome.map_crc != building.crc:
		net.close()
		start.set_busy(false)
		start.set_status("Niezgodna wersja mapy (serwer %08x, klient %08x)" % [welcome.map_crc, building.crc], true)
		return
	_show_portal()
	if game:
		game.reset_session(welcome)  # auto-reconnect: keep the world, new session
		return
	start.get_parent().visible = false
	get_window().title = "Startup Sim — %s" % net.nick
	game = Game.new()
	add_child(game)
	game.setup(net, building, welcome, net.nick, args)
	game.set_own_appearance(profile.appearance)
	game.entered_world.connect(func(): portal.on_entered_world(); _sync_portal())
	_sync_portal()


## New session: the server starts us on the job portal (unless it runs with
## --skip-recruitment, then snapshots arrive and the portal closes itself).
func _show_portal() -> void:
	portal.set_profile(net.nick, profile)
	portal.reset()
	_sync_portal()


func _sync_portal() -> void:
	portal_layer.visible = portal.visible
	if game:
		game.input_blocked = portal.visible
		game.set_job(portal.job_title, portal.department)


func _on_packet(p: Dictionary) -> void:
	portal.on_packet(p)
	if p.type == Protocol.T_RECRUIT_RESULT:
		_sync_portal()


func _on_reconnecting(reason: String) -> void:
	if game:
		game.on_reconnecting(reason)


func _on_disconnected(reason: String) -> void:
	portal_layer.visible = false
	if game:
		game.queue_free()
		game = null
	start.get_parent().visible = true
	start.set_busy(false)
	start.set_status(reason, true)


func _notification(what: int) -> void:
	if what == NOTIFICATION_WM_CLOSE_REQUEST:
		net.close()
		get_tree().quit()
