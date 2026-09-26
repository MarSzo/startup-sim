## Entry point: character creation <-> game. User args (after `--`):
##   --nick=Ala --server=127.0.0.1:7777 --autoconnect --debug --autowalk
##   --screenshot=/path.png [--screenshot-delay=5]  (dev: save a frame and quit;
##     several delays "5,12,20" save path_1.png, path_2.png, ... and quit after the last)
##   --commute=3  (dev: pick this way to work every morning; 1 foot .. 5 tram)
##   --auto-recruit=1 [--auto-recruit-delay=2]  (dev: apply for offer 1, answer
##     at random until hired, waiting N s before each click)
extends Node

const NetClient = preload("res://net/net_client.gd")
const Building = preload("res://map/building.gd")
const Game = preload("res://game/game.gd")
const CharacterScreen = preload("res://ui/character_screen.gd")
const Ink = preload("res://ui/ink_ui.gd")
const Desktop = preload("res://ui/desktop.gd")
const Protocol = preload("res://net/protocol.gd")
const DayScreen = preload("res://ui/day_screen.gd")
const TitleScreen = preload("res://ui/title_screen.gd")
const PauseMenu = preload("res://ui/pause_menu.gd")
const Settings = preload("res://ui/settings.gd")

const BUILDING_PATH := "res://maps/building.json"

var args := {}
var net := NetClient.new()
var building
var start := CharacterScreen.new()
var profile := {}
var game: Node = null
var portal_layer := CanvasLayer.new()
var portal := Desktop.new()
var day_layer := CanvasLayer.new()
var day_screen := DayScreen.new()
var title_layer := CanvasLayer.new()
var title := TitleScreen.new()
var pause_layer := CanvasLayer.new()
var pause := PauseMenu.new()
var _leaving := false  # "Wyjdź do menu": the disconnect goes to the title


func _ready() -> void:
	# Pixel font and frames everywhere (also text drawn with the fallback font).
	ThemeDB.fallback_font = Ink.font()
	ThemeDB.fallback_font_size = 16
	ThemeDB.get_default_theme().merge_with(Ink.theme())
	get_tree().root.theme = Ink.theme()
	for a in OS.get_cmdline_user_args():
		var kv: PackedStringArray = a.trim_prefix("--").split("=", true, 1)
		args[kv[0]] = kv[1] if kv.size() > 1 else ""
	get_tree().auto_accept_quit = false
	Settings.load_once()
	Settings.apply_window()
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
	day_layer.layer = 30  # above the world and the home computer
	add_child(day_layer)
	day_layer.add_child(day_screen)
	day_screen.choose_commute.connect(func(m: int): net.send(Protocol.encode_commute_choice(net.token, m)))
	portal.auto_offer = int(args.get("auto-recruit", "0"))
	portal.auto_delay = float(args.get("auto-recruit-delay", "0"))
	portal.apply.connect(func(offer, motivation): net.send(Protocol.encode_apply(net.token, offer, motivation)))
	portal.answer.connect(func(a, i, c): net.send(Protocol.encode_answer(net.token, a, i, c)))
	portal.portal_action.connect(func(action, arg): net.send(Protocol.encode_portal_action(net.token, action, arg)))
	portal.found_company.connect(func(name): net.send(Protocol.encode_company_action(net.token, Protocol.CO_FOUND, 0, 0, name)))
	portal.auto_found = args.get("found", "")
	var ui := CanvasLayer.new()
	add_child(ui)
	ui.add_child(start)
	start.connect_pressed.connect(_on_connect_pressed)
	start.back_pressed.connect(_show_title)
	# Title screen (skipped by the dev --nick / --autoconnect) and the Esc menu.
	title_layer.layer = 40
	add_child(title_layer)
	title_layer.add_child(title)
	title.play.connect(func():
		title_layer.visible = false
		start.get_parent().visible = true)
	title.quit.connect(_quit)
	pause_layer.layer = 50
	add_child(pause_layer)
	pause_layer.add_child(pause)
	pause.to_menu.connect(_leave_to_menu)
	pause.quit.connect(_quit)
	pause.settings_changed.connect(func(): if game: game.apply_settings())
	portal.menu_requested.connect(func(): pause.open())
	if args.has("nick") or args.has("autoconnect"):
		title_layer.visible = false
	else:
		ui.visible = false
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
	get_viewport().gui_release_focus()  # the nick field must not keep eating keys
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
		game.input_blocked = portal.visible or day_screen.blocking() or pause.visible
		game.set_job(portal.job_title, portal.department)


func _on_packet(p: Dictionary) -> void:
	portal.on_packet(p)
	if p.type == Protocol.T_RECRUIT_RESULT:
		_sync_portal()
	if p.type == Protocol.T_CLOCK:
		portal.game_day = p.day
		portal.game_minute = p.minute
		portal.on_clock(p)
		day_screen.on_clock(p)
		var want := int(args.get("commute", "0"))
		if want > 0 and p.place == Protocol.PLACE_COMMUTING and p.arrive == Protocol.NO_TIME and p.mode != want:
			net.send(Protocol.encode_commute_choice(net.token, want))
		_sync_portal()


func _process(_d: float) -> void:
	if game:
		game.input_blocked = portal.visible or day_screen.blocking() or pause.visible  # no walking under the menu


func _show_title() -> void:
	start.get_parent().visible = false
	title_layer.visible = true


## Esc: the game menu (unless a game window wants the key to close itself).
func _input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and not event.echo and event.keycode == KEY_ESCAPE:
		if game and not pause.visible and not title_layer.visible and not game.window_open():
			pause.open()
			get_viewport().set_input_as_handled()


## "Wyjdź do menu": leave the server, back to the title.
func _leave_to_menu() -> void:
	_leaving = true
	net.close()
	_end_game()
	start.set_busy(false)
	start.set_status("")
	_show_title()
	_leaving = false


func _end_game() -> void:
	portal_layer.visible = false
	day_screen.visible = false
	if game:
		game.queue_free()
		game = null
	get_window().title = "Startup Sim"


func _quit() -> void:
	net.close()
	get_tree().quit()


func _on_reconnecting(reason: String) -> void:
	if game:
		game.on_reconnecting(reason)


func _on_disconnected(reason: String) -> void:
	if _leaving:
		return
	_end_game()
	start.get_parent().visible = true
	start.set_busy(false)
	start.set_status(reason, true)


func _notification(what: int) -> void:
	if what == NOTIFICATION_WM_CLOSE_REQUEST:
		net.close()
		get_tree().quit()
