## Entry point: character creation <-> game. User args (after `--`):
##   --nick=Ala --server=127.0.0.1:7777 --autoconnect --debug --autowalk
##   --screenshot=/path.png [--screenshot-delay=5]  (dev: save a frame and quit;
##     several delays "5,12,20" save path_1.png, path_2.png, ... and quit after the last)
##   --record=/dir [--record-start=2 --record-length=6 --record-fps=30]  (dev:
##     JPG frames for a trailer, see dev_recorder.gd; quits when done)
##   --login=Nick:haslo [--register] [--server=host:port]  (dev: log in / sign up
##     through the login screen at once)
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
const Audio = preload("res://audio/audio.gd")
const AuthClient = preload("res://net/auth_client.gd")
const LoginScreen = preload("res://ui/login_screen.gd")

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
var audio := Audio.new()
var auth := AuthClient.new()
var login_layer := CanvasLayer.new()
var login := LoginScreen.new()
## The logged-in account: {address, nick, ticket, refresh, key} ({} = a
## guest). `key` seals the game packets; it only ever comes over HTTPS.
var session := {}
var _retry_login := false   # the ticket expired during a reconnect: refresh it
var _to_login := ""         # refused (needs an account): back to the login screen
var _to_char := ""          # refused (the e-mail / nick is taken): fix it on the character screen
## A character that exists on the server: any valid profile will do (the
## server keeps and sends the real one).
const STUB_PROFILE := {"gender": 2, "age": 25, "city": "Kraków", "email": "postac@startup.sim",
	"appearance": {"skin": 0, "hair_style": 0, "hair_color": 0, "shirt": 1, "pants": 0}}
var _last_place := -1


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
	Settings.apply_fps()
	add_child(audio)
	Settings.apply_audio()
	# Every button in the game clicks.
	get_tree().node_added.connect(func(n: Node):
		if n is BaseButton:
			n.pressed.connect(func(): audio.play("ui_click", -6.0, 0.08)))
	building = Building.new()
	building.load_path(BUILDING_PATH)
	add_child(auth)
	add_child(net)
	net.rejected.connect(_on_rejected)
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
	day_screen.skip_wait.connect(func(): net.send(Protocol.encode_skip_wait(net.token)))
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
		login_layer.visible = true)
	# Logging in (between the title and the game).
	login_layer.layer = 36
	login_layer.visible = false
	add_child(login_layer)
	login.auth = auth
	login_layer.add_child(login)
	login.back.connect(_show_title)
	login.logged_in.connect(_on_logged_in)
	title.quit.connect(_quit)
	pause_layer.layer = 50
	add_child(pause_layer)
	pause_layer.add_child(pause)
	pause.to_menu.connect(_leave_to_menu)
	pause.logout.connect(_logout)
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
	if args.has("login-screen"):  # dev: straight to the login screen (=form: the form)
		title_layer.visible = false
		login_layer.visible = true
		if args["login-screen"] == "form":
			login._show_form.call_deferred()
	if args.has("login"):
		var np: PackedStringArray = str(args["login"]).split(":", true, 1)
		title_layer.visible = false
		login_layer.visible = true
		login._show_form()
		login.set_address(args.get("server", AuthClient.DEFAULT_SERVER))
		login.nick_edit.text = np[0]
		login.pass_edit.text = np[1] if np.size() > 1 else ""
		login._go.call_deferred("register" if args.has("register") else "login")
	if args.has("record"):
		var rec := preload("res://dev_recorder.gd").new()
		rec.dir = args["record"]
		rec.start = float(args.get("record-start", "2"))
		rec.length = float(args.get("record-length", "6"))
		rec.fps = float(args.get("record-fps", "30"))
		add_child(rec)
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


## Logged in: straight into the game with a saved character, else create one.
func _on_logged_in(address: String, g: Dictionary, remember: bool) -> void:
	session = {"address": address, "nick": g.nick, "ticket": g.ticket, "refresh": g.refresh, "key": g.get("key", "")}
	if remember:
		AuthClient.remember(address, g.nick, g.refresh)
	else:
		AuthClient.forget()
	login_layer.visible = false
	if g.get("character", false):
		start.get_parent().visible = true
		_on_connect_pressed(g.nick, STUB_PROFILE, address)
	else:
		start.get_parent().visible = true
		start.set_busy(false)
		start.for_account(g.nick, address)
		if args.has("autocreate"):  # dev: accept the character as it is
			start._submit.call_deferred()


func _on_connect_pressed(nick: String, p_profile: Dictionary, address: String) -> void:
	profile = p_profile
	var err: String = net.connect_to_server(address, nick, p_profile, session.get("ticket", ""), session.get("key", ""))
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
		if p.place == Protocol.PLACE_HOME and _last_place != Protocol.PLACE_HOME and _last_place >= 0 and p.pay > 0:
			audio.play("coin", -4.0)  # payday
		_last_place = p.place
		var want := int(args.get("commute", "0"))
		if want > 0 and p.place == Protocol.PLACE_COMMUTING and p.arrive == Protocol.NO_TIME and p.mode != want:
			net.send(Protocol.encode_commute_choice(net.token, want))
		_sync_portal()


func _process(_d: float) -> void:
	if args.has("perf"):
		_perf_report()
	if game:
		game.input_blocked = portal.visible or day_screen.blocking() or pause.visible  # no walking under the menu
	# Music: the menu tune on the title / character screens, a calm one at
	# home (desktop, night, the way to work); in the office only ambience.
	if title_layer.visible or start.get_parent().visible:
		audio.music("music_menu")
	elif portal_layer.visible or day_screen.blocking():
		audio.music("music_home")
	else:
		audio.music("")


var _perf_at := 0
var _perf_frames := 0
var _perf_proc := 0.0
var _perf_phys := 0.0
var _perf_draws := {}   # script / class -> redraws in this period
var _perf_hooked := {}  # instance id -> true


func _perf_hook(n: Node) -> void:
	if n is CanvasItem and not _perf_hooked.has(n.get_instance_id()):
		_perf_hooked[n.get_instance_id()] = true
		var key: String = n.get_script().resource_path.get_file() if n.get_script() else n.get_class()
		n.draw.connect(func(): _perf_draws[key] = _perf_draws.get(key, 0) + 1)
	for c in n.get_children():
		_perf_hook(c)


## --perf: every 2 s, the frame's script and engine cost (for benchmarks).
func _perf_report() -> void:
	_perf_frames += 1
	_perf_proc += Performance.get_monitor(Performance.TIME_PROCESS)
	_perf_phys += Performance.get_monitor(Performance.TIME_PHYSICS_PROCESS)
	var now := Time.get_ticks_msec()
	if now - _perf_at < 2000:
		return
	_perf_hook(self)
	if _perf_at > 0:
		var top := _perf_draws.keys()
		top.sort_custom(func(a, b): return _perf_draws[a] > _perf_draws[b])
		var parts := []
		for k in top.slice(0, 6):
			parts.append("%s %.0f/s" % [k, _perf_draws[k] / ((now - _perf_at) / 1000.0)])
		print("perf: redraws  " + ", ".join(parts))
		print("perf: fps %d  process %.2f ms  physics %.2f ms  draw calls %d  items %d  nodes %d" % [
			Engine.get_frames_per_second(), 1000.0 * _perf_proc / _perf_frames, 1000.0 * _perf_phys / _perf_frames,
			Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME),
			Performance.get_monitor(Performance.RENDER_TOTAL_OBJECTS_IN_FRAME),
			Performance.get_monitor(Performance.OBJECT_NODE_COUNT)])
	_perf_at = now
	_perf_draws.clear()
	_perf_frames = 0
	_perf_proc = 0.0
	_perf_phys = 0.0


func _show_title() -> void:
	start.get_parent().visible = false
	login_layer.visible = false
	title_layer.visible = true


func _show_login(message: String) -> void:
	start.get_parent().visible = false
	title_layer.visible = false
	login_layer.visible = true
	login.set_status(message, message != "")


## The server refused us: an expired ticket gets refreshed (and we come
## back in); "needs an account" sends us to the login screen.
func _on_rejected(reason: int) -> void:
	var msg: String = Protocol.REJECT_REASONS.get(reason, "")
	if reason == Protocol.REJECT_BAD_TICKET and session.get("refresh", "") != "":
		_retry_login = true
	elif reason == Protocol.REJECT_EMAIL_TAKEN or (reason == Protocol.REJECT_NICK_TAKEN and session.is_empty()):
		_to_char = msg
	elif reason in [Protocol.REJECT_BAD_TICKET, Protocol.REJECT_GUESTS_OFF, Protocol.REJECT_NICK_TAKEN]:
		_to_login = msg


func _logout() -> void:
	if not session.is_empty():
		auth.logout(session.address, session.refresh)
	AuthClient.forget()
	session = {}
	_leaving = true
	net.close()
	_end_game()
	_leaving = false
	_show_login("")
	login.set_status("Wylogowano.")


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
		audio.leave_world()
		game.queue_free()
		game = null
	_last_place = -1
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
	if _to_char != "":
		# Taken e-mail / nick: back to the character, the login still holds.
		start.get_parent().visible = true
		start.set_busy(false)
		start.set_status(_to_char, true)
		_to_char = ""
		return
	if _retry_login:
		# The login ticket expired while reconnecting: a fresh one, then back in.
		_retry_login = false
		var r: Dictionary = await auth.refresh(session.address, session.refresh)
		if r.get("ok", false):
			session.ticket = r.ticket
			session.refresh = r.refresh
			session.key = r.get("key", "")
			if AuthClient.remembered().get("nick", "") == session.nick:
				AuthClient.remember(session.address, session.nick, r.refresh)
			start.get_parent().visible = true
			_on_connect_pressed(session.nick, STUB_PROFILE, session.address)
			return
		session = {}
		_show_login(str(r.get("error", reason)))
		return
	if _to_login != "" or not session.is_empty():
		var msg := _to_login if _to_login != "" else reason
		_to_login = ""
		start.set_busy(false)
		_show_login(msg)
		return
	start.get_parent().visible = true
	start.set_busy(false)
	start.set_status(reason, true)


func _notification(what: int) -> void:
	match what:
		NOTIFICATION_WM_CLOSE_REQUEST:
			net.close()
			get_tree().quit()
		NOTIFICATION_APPLICATION_FOCUS_OUT:
			Settings.apply_fps(false)  # in the background: draw less
		NOTIFICATION_APPLICATION_FOCUS_IN:
			Settings.apply_fps(true)
