## Entry point: start screen <-> game. User args (after `--`):
##   --nick=Ala --server=127.0.0.1:7777 --autoconnect --debug --autowalk
##   --screenshot=/path.png [--screenshot-delay=5]  (dev: save a frame and quit)
extends Node

const NetClient = preload("res://net/net_client.gd")
const MapData = preload("res://map/map_data.gd")
const Game = preload("res://game/game.gd")
const StartScreen = preload("res://ui/start_screen.gd")

const MAP_PATH := "res://maps/floor0.json"

var args := {}
var net := NetClient.new()
var map
var start := StartScreen.new()
var game: Node = null


func _ready() -> void:
	for a in OS.get_cmdline_user_args():
		var kv: PackedStringArray = a.trim_prefix("--").split("=", true, 1)
		args[kv[0]] = kv[1] if kv.size() > 1 else ""
	get_tree().auto_accept_quit = false
	map = MapData.new()
	map.load_path(MAP_PATH)
	add_child(net)
	net.connected.connect(_on_connected)
	net.disconnected.connect(_on_disconnected)
	var ui := CanvasLayer.new()
	add_child(ui)
	ui.add_child(start)
	start.connect_pressed.connect(_on_connect_pressed)
	start.set_defaults(args.get("nick", "Gracz%d" % randi_range(100, 999)), args.get("server", "127.0.0.1:7777"))
	if map.error != "":
		start.set_status("Błąd mapy: " + map.error, true)
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
	if welcome.map_crc != map.crc:
		net.close()
		start.set_busy(false)
		start.set_status("Niezgodna wersja mapy (serwer %08x, klient %08x)" % [welcome.map_crc, map.crc], true)
		return
	start.get_parent().visible = false
	get_window().title = "Startup Sim — %s" % net.nick
	game = Game.new()
	add_child(game)
	game.setup(net, map, welcome, net.nick, args)


func _on_disconnected(reason: String) -> void:
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
