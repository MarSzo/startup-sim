## Entry point: start screen <-> game. User args (after `--`):
##   --nick=Ala --server=127.0.0.1:7777 --autoconnect --debug --autowalk
##   --screenshot=/path.png [--screenshot-delay=5]  (dev: save a frame and quit)
##   --auto-recruit=1 [--auto-recruit-delay=2]  (dev: apply for offer 1, answer
##     at random until hired, waiting N s before each click)
extends Node

const NetClient = preload("res://net/net_client.gd")
const Building = preload("res://map/building.gd")
const Game = preload("res://game/game.gd")
const StartScreen = preload("res://ui/start_screen.gd")
const Portal = preload("res://ui/portal.gd")
const Protocol = preload("res://net/protocol.gd")

const BUILDING_PATH := "res://maps/building.json"

var args := {}
var net := NetClient.new()
var building
var start := StartScreen.new()
var game: Node = null
var portal_layer := CanvasLayer.new()
var portal := Portal.new()


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
	portal.apply.connect(func(offer): net.send(Protocol.encode_apply(net.token, offer)))
	portal.answer.connect(func(a, i, c): net.send(Protocol.encode_answer(net.token, a, i, c)))
	portal.done.connect(_on_portal_done)
	var ui := CanvasLayer.new()
	add_child(ui)
	ui.add_child(start)
	start.connect_pressed.connect(_on_connect_pressed)
	start.set_defaults(args.get("nick", "Gracz%d" % randi_range(100, 999)), args.get("server", "127.0.0.1:7777"))
	if building.error != "":
		start.set_status("Błąd mapy: " + building.error, true)
		start.set_busy(true)
	elif args.has("autoconnect"):
		_on_connect_pressed(start.nick_edit.text, start.addr_edit.text)
	if args.has("screenshot"):
		_take_screenshot(args["screenshot"], float(args.get("screenshot-delay", "5")))


func _take_screenshot(path: String, delay: float) -> void:
	await get_tree().create_timer(delay).timeout
	await RenderingServer.frame_post_draw
	get_viewport().get_texture().get_image().save_png(path)
	print("screenshot saved: ", path)
	if game:
		print(game.debug_text())
	net.close()
	get_tree().quit()


func _on_connect_pressed(nick: String, address: String) -> void:
	var err: String = net.connect_to_server(address, nick)
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
	game.entered_world.connect(func(): portal.on_entered_world(); _sync_portal())
	_sync_portal()


## New session: the server starts us on the job portal (unless it runs with
## --skip-recruitment, then snapshots arrive and the portal closes itself).
func _show_portal() -> void:
	portal.reset()
	_sync_portal()


func _sync_portal() -> void:
	portal_layer.visible = portal.visible
	if game:
		game.input_blocked = portal.visible
		game.set_job(portal.job_title, portal.department)


func _on_portal_done() -> void:
	_sync_portal()


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
