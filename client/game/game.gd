## In-game world: local prediction + reconciliation, remote interpolation,
## floors and room-based visibility, camera and debug info.
extends Node2D

## First snapshot of a session arrived: we are in the world.
signal entered_world

const Protocol = preload("res://net/protocol.gd")
const Movement = preload("res://sim/movement.gd")
const MapView = preload("res://map/map_view.gd")
const PlayerView = preload("res://game/player_view.gd")
const RemotePlayer = preload("res://game/remote_player.gd")
const DebugOverlay = preload("res://ui/debug_overlay.gd")
const MapData = preload("res://map/map_data.gd")
const ItemArt = preload("res://game/item_art.gd")
const ItemView = preload("res://game/item_view.gd")
const InventoryHud = preload("res://ui/inventory_hud.gd")
const ComputerView = preload("res://game/computer_view.gd")
const ComputerScreen = preload("res://ui/computer_screen.gd")
const StatsHud = preload("res://ui/stats_hud.gd")
const StallDoorView = preload("res://game/stall_door_view.gd")
const ElevatorDoorView = preload("res://game/elevator_door_view.gd")
const RideMask = preload("res://game/ride_mask.gd")
const ShelfWindow = preload("res://ui/shelf_window.gd")
const FridgeWindow = preload("res://ui/fridge_window.gd")
const VehicleView = preload("res://game/vehicle_view.gd")
const WeatherFx = preload("res://ui/weather_fx.gd")
const DialogWindow = preload("res://ui/dialog_window.gd")
const TrayView = preload("res://game/tray_view.gd")
const SmokeView = preload("res://game/smoke_view.gd")
const LightView = preload("res://game/light_view.gd")
const Settings = preload("res://ui/settings.gd")
const Ink = preload("res://ui/ink_ui.gd")

const ZOOM := 3.0
## Remote players are rendered this far in the past (2 snapshots at 20 Hz).
const INTERP_DELAY_SEC := 0.1
## Each Input packet repeats this many latest inputs (covers packet loss).
const INPUT_REDUNDANCY := 4
## Remote players missing from snapshots for this many ticks are removed.
const REMOTE_TIMEOUT_TICKS := 5
## Visual correction error decays with this rate (1/s).
const ERROR_DECAY := 15.0
const MAX_PENDING := 240
## Talk range to NPCs (same as npc::TALK_RADIUS on the server): 3.5 tiles.
const TALK_RADIUS_PX := 56.0
const LOG_LINES := 4
const LOG_TTL_SEC := 12.0
const DEPT_SHORT := {1: "IT", 2: "Biznes", 3: "Zarząd"}
const DEPT_NAMES := {1: "IT / Produkt", 2: "Biznes", 3: "Zarząd"}

var net
var building
var views := {}          # floor -> MapView (only the current floor is visible)
var tick_hz := 20
var nick := ""
var world := Node2D.new()
var sounds := preload("res://audio/game_sounds.gd").new()
var me := PlayerView.new()
var camera := Camera2D.new()
var overlay := DebugOverlay.new()
var status_layer := CanvasLayer.new()
var status_label := Label.new()
var hint_label := Label.new()
var log_label := Label.new()
var _log: Array = []  # [msec, text]
var kinds := {}          # id -> entity kind (player / NPC)
var floor_items := {}    # entity id -> ItemView (items lying on the floor)
var inventory: Array = [] # hands + pockets (from the server)
var hud := InventoryHud.new()
var computers := {}      # entity id -> ComputerView (laptops on desks)
var vehicles := {}       # entity id -> VehicleView (cars, bikes, taxis, trams)
var commute_mode := 5    # how we came today (Clock.mode): where "[E] home" is
var trays := {}          # entity id -> TrayView (sweets in the chill room)
var screen := ComputerScreen.new()
var screen_layer := CanvasLayer.new()
var stats_hud := StatsHud.new()
var stall_doors := {}    # floor -> Array of StallDoorView
var elevator_doors := {} # floor -> Array of ElevatorDoorView
var lift_floor := 0      # elevator: where the car is (from Doors)
var lift_target := 255   # ...and where it is heading (Protocol.NO_FLOOR = standing)
var lift_moving := false
var ride_mask := RideMask.new()
var clock_label := Label.new()
var daylight := CanvasModulate.new()   # time-of-day tint of the world
var game_minute := 8 * 60
var weather := Protocol.WEATHER_SUNNY
var weather_fx := WeatherFx.new()
## Smoke in the rooms + detectors (over the world); fire alarm on screen.
var smoke_view := SmokeView.new()
## Room brightness (windows, lamps, time of day, weather) and the switches.
var light_view := LightView.new()
var fire_alarm := false
var alarm_tint := ColorRect.new()
var alarm_label := Label.new()
## The world's paper-and-ink look (post-process over the world).
var mood_layer := CanvasLayer.new()
var mood := ColorRect.new()
## Text in the world (nicks, bubbles, room names) above the ink effect.
var label_layer := CanvasLayer.new()
var smoke_layer := CanvasLayer.new()
var weather_layer := CanvasLayer.new()
var dialog := DialogWindow.new()
var shelf_window := ShelfWindow.new()
var _shelf_at := Vector2.ZERO     # where the shelf window was opened (walk away = close)
var fridge_window := FridgeWindow.new()
var _fridge_at := Vector2.ZERO
var depts := {}          # id -> department (after the contract)
var appearances := {}    # id -> appearance dict (from PlayerInfo)
var own_appearance := {}
## Set while a full-screen UI (job portal) is open: no movement input.
var input_blocked := false
var job_title := ""
var department := 0
var _pending_say := {}   # id -> [msec, text]: said before the speaker was visible
var remotes := {}        # id -> RemotePlayer
var nicks := {}          # id -> String
var info_requested := {} # id -> msec of last request

# Local prediction state: pred is a Movement.body() (floor, pos, prev, lock).
var have_state := false
var pred := {}
var prev_pos := Vector2i.ZERO   # position one physics step ago (render lerp)
var pending: Array = []  # [seq, bits], oldest first
var seq := 0
var last_ack := 0
var error_offset := Vector2.ZERO
var corrections := 0

# Server time / snapshot state.
var latest_tick := 0
var est_tick := 0.0
var have_time := false
var room_id := 0
var floor_index := 0
var visible_count := 0
var interp_frames := 0
var interp_underruns := 0

# Dev helpers: --autowalk (random walk), --goto=<leg>;<leg>;... where a leg is
# a room name or "x,y" tile on the current floor, "E" (press interact once) or
# "wait:N" (stand still N seconds). E.g. "27,29;E;wait:2;34,6;Recepcja".
var autowalk := false
var _autowalk_bits := 0
var _autowalk_timer := 0.0
var goto_legs: PackedStringArray = []
var goto_delay := 3.0
var _goto_path: Array[Vector2i] = []
var _goto_floor := -1   # floor the current path was planned on


func setup(p_net, p_building, welcome: Dictionary, p_nick: String, args: Dictionary) -> void:
	RenderingServer.set_default_clear_color(Color("#15171f"))  # the night around the building
	label_layer.layer = 7
	label_layer.follow_viewport_enabled = true
	add_child(label_layer)
	PlayerView.label_root = label_layer
	net = p_net
	building = p_building
	nick = p_nick
	tick_hz = welcome.tick_hz
	autowalk = args.has("autowalk")
	if args.get("goto", "") != "":
		goto_legs = args["goto"].split(";")
	goto_delay = float(args.get("goto-delay", "3"))
	net.packet_received.connect(_on_packet)
	sounds.game = self
	add_child(sounds)
	var audio = preload("res://audio/audio.gd").inst
	if audio:
		audio.world = world

	var map0 = building.get_floor(0)
	for f in building.floors.size():
		var m = building.get_floor(f)
		if m == null:
			continue
		var view := MapView.new()
		var names := {}
		for g in building.floors.size():
			names[g] = building.floor_name(g)
		view.build(m, ZOOM, names)
		view.visible = false
		add_child(view)
		view.remove_child(view.labels)
		label_layer.add_child(view.labels)
		view.labels.visible = false
		views[f] = view
	world.y_sort_enabled = true
	add_child(ride_mask)  # between the map and the people
	add_child(world)
	light_view.setup(building)
	add_child(light_view)  # over the world (and inked with it)
	# Smoke over the ink effect (drawn in its own style), under the weather.
	smoke_layer.layer = 5
	smoke_layer.follow_viewport_enabled = true
	add_child(smoke_layer)
	smoke_view.setup(building)
	smoke_layer.add_child(smoke_view)
	for f in views:
		var m = building.get_floor(f)
		stall_doors[f] = []
		elevator_doors[f] = []
		for y in m.height:
			for x in m.width:
				var ttype = m.legend.get(m.tile_chars[y * m.width + x], {}).get("type")
				if ttype == "elevator_door":
					var ev := ElevatorDoorView.new()
					ev.tile = Vector2i(x, y)
					ev.position = Movement.to_px(Movement.tile_center(x, y))
					var is_door := func(dx: int) -> bool: return m.legend.get(m.tile_chars[y * m.width + x + dx], {}).get("type") == "elevator_door"
					ev.display = is_door.call(-1) and is_door.call(1)  # the middle one
					ev.visible = false
					world.add_child(ev)
					elevator_doors[f].append(ev)
				if ttype == "stall_door":
					var dv := StallDoorView.new()
					dv.tile = Vector2i(x, y)
					dv.position = Movement.to_px(Movement.tile_center(x, y))
					dv.visible = false
					world.add_child(dv)
					stall_doors[f].append(dv)

	me.setup(net.player_id, nick, ZOOM)
	me.highlight = true
	me.visible = false
	world.add_child(me)
	camera.zoom = Vector2(ZOOM, ZOOM)
	camera.limit_left = 0
	camera.limit_top = 0
	camera.limit_right = map0.width * map0.tile_px
	camera.limit_bottom = map0.height * map0.tile_px
	me.add_child(camera)
	camera.make_current()

	add_child(overlay)
	overlay.game = self
	overlay.visible = args.has("debug")

	status_layer.layer = 11
	add_child(status_layer)
	status_label.set_anchors_preset(Control.PRESET_CENTER_TOP)
	status_label.position = Vector2(-200, 24)
	status_label.size = Vector2(400, 40)
	status_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	status_label.add_theme_font_size_override("font_size", 24)
	status_label.add_theme_constant_override("outline_size", 6)
	status_label.add_theme_color_override("font_outline_color", Color.BLACK)
	status_label.visible = false
	status_layer.add_child(status_label)
	hint_label.set_anchors_preset(Control.PRESET_CENTER_BOTTOM)
	hint_label.position = Vector2(-250, -214)  # above the inventory bar
	hint_label.size = Vector2(500, 36)
	hint_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	hint_label.add_theme_font_size_override("font_size", 20)
	hint_label.add_theme_constant_override("outline_size", 6)
	hint_label.add_theme_color_override("font_outline_color", Color.BLACK)
	hint_label.visible = false
	status_layer.add_child(hint_label)
	log_label.set_anchors_preset(Control.PRESET_BOTTOM_LEFT)
	log_label.position = Vector2(16, -140)
	log_label.size = Vector2(430, 124)
	log_label.vertical_alignment = VERTICAL_ALIGNMENT_BOTTOM
	log_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	log_label.add_theme_font_size_override("font_size", 16)
	log_label.add_theme_constant_override("line_spacing", 2)
	log_label.add_theme_constant_override("outline_size", 5)
	log_label.add_theme_color_override("font_outline_color", Color.BLACK)
	status_layer.add_child(log_label)
	status_layer.add_child(hud)
	hud.slot_clicked.connect(_pocket_key)
	status_layer.add_child(stats_hud)
	var cp := Ink.panel("hud")
	cp.position = Vector2(16, 16)
	Ink.style_label(clock_label, 20, Ink.TEXT)
	cp.add_child(clock_label)
	status_layer.add_child(cp)
	add_child(daylight)
	mood_layer.layer = 4  # over the world, under the weather and the HUD
	add_child(mood_layer)
	mood.set_anchors_preset(Control.PRESET_FULL_RECT)
	mood.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var mat := ShaderMaterial.new()
	mat.shader = load("res://game/mood.gdshader")
	mood.material = mat
	Settings.load_once()
	mood.visible = Settings.mood and not args.has("no-mood")
	mood_layer.add_child(mood)
	alarm_tint.set_anchors_preset(Control.PRESET_FULL_RECT)
	alarm_tint.color = Color(0.9, 0.05, 0.05, 0.0)
	alarm_tint.mouse_filter = Control.MOUSE_FILTER_IGNORE
	status_layer.add_child(alarm_tint)
	status_layer.move_child(alarm_tint, 0)  # under the HUD
	alarm_label.set_anchors_preset(Control.PRESET_CENTER_TOP)
	alarm_label.position = Vector2(-330, 70)
	alarm_label.size = Vector2(660, 40)
	alarm_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	alarm_label.text = "ALARM POŻAROWY — wyjdź z budynku!"
	alarm_label.add_theme_font_size_override("font_size", 32)
	alarm_label.add_theme_color_override("font_color", Color("#ffdddd"))
	alarm_label.add_theme_constant_override("outline_size", 8)
	alarm_label.add_theme_color_override("font_outline_color", Color("#7a0000"))
	alarm_label.visible = false
	status_layer.add_child(alarm_label)
	weather_layer.layer = 6  # over the world and the smoke, under the HUD
	add_child(weather_layer)
	weather_layer.add_child(weather_fx)
	weather_fx.lightning.connect(func(): sounds.on_lightning(weather_fx.outdoors))
	status_layer.add_child(shelf_window)
	status_layer.add_child(fridge_window)
	fridge_window.action.connect(func(act: int, arg: int): if net.is_playing(): net.send(Protocol.encode_fridge_action(net.token, act, arg)))
	shelf_window.take.connect(func(shelf: int, kind: int): if net.is_playing(): net.send(Protocol.encode_shop_take(net.token, shelf, kind)))
	screen_layer.layer = 12
	add_child(screen_layer)
	screen.my_id = net.player_id
	screen.name_of = func(id: int) -> String: return nick if id == net.player_id else nicks.get(id, "?")
	screen.action.connect(_computer_action)
	screen.company_action.connect(func(act: int, target: int, value: int, text: String):
		if net.is_playing():
			net.send(Protocol.encode_company_action(net.token, act, target, value, text)))
	screen.order.connect(func(dish: int): if net.is_playing(): net.send(Protocol.encode_lunch_order(net.token, dish)))
	screen.book.connect(func(start: int, topic: int): if net.is_playing(): net.send(Protocol.encode_calendar_book(net.token, start, topic)))
	status_layer.add_child(dialog)
	dialog.name_of = func(id: int) -> String: return nicks.get(id, "?")
	dialog.answer.connect(func(id: int, choice: int): if net.is_playing(): net.send(Protocol.encode_dialog_answer(net.token, id, choice)))
	screen_layer.add_child(screen)
	_show_floor(0)
	set_zoom_level.call_deferred(float(args["zoom"]) if args.has("zoom") else Settings.zoom)


func _show_floor(f: int) -> void:
	smoke_view.set_floor(f)
	light_view.set_floor(f)
	_below_floor = -1
	for k in views:
		views[k].visible = (k == f)
		views[k].labels.visible = (k == f)
	for k in stall_doors:
		for dv in stall_doors[k]:
			dv.visible = (k == f)
	for k in elevator_doors:
		for ev in elevator_doors[k]:
			ev.visible = (k == f)


## Session lost; the net client is getting a new one. Freeze local simulation.
func on_reconnecting(reason: String) -> void:
	have_state = false
	status_label.text = "Łączenie ponownie… (%s)" % reason if reason != "" else "Łączenie ponownie…"
	status_label.visible = true


## New session after an automatic reconnect: new player id/token, fresh state.
func reset_session(welcome: Dictionary) -> void:
	tick_hz = welcome.tick_hz
	for r in remotes.values():
		r.queue_free()
	remotes.clear()
	nicks.clear()
	kinds.clear()
	for iv in floor_items.values():
		iv.queue_free()
	floor_items.clear()
	for cv in computers.values():
		cv.queue_free()
	computers.clear()
	screen.set_seated(false)
	screen.chats.clear()
	for f in views:
		building.get_floor(f).set_closed_tiles([])
	screen.my_id = net.player_id
	inventory = []
	hud.update_slots([])
	me.set_held(0)
	appearances.clear()
	depts.clear()
	info_requested.clear()
	pending.clear()
	seq = 0
	last_ack = 0
	error_offset = Vector2.ZERO
	have_state = false
	have_time = false
	latest_tick = 0
	room_id = 0
	me.visible = false
	me.set_seed(net.player_id)
	if not own_appearance.is_empty():
		me.set_appearance(own_appearance)
	status_label.visible = false


func set_own_appearance(a: Dictionary) -> void:
	own_appearance = a
	me.set_appearance(a)


## Position from the job portal (department becomes official with the contract).
func set_job(title: String, dept: int) -> void:
	job_title = title
	department = dept
	_refresh_own_label()


func _label_for(nick_text: String, dept: int) -> String:
	return "%s · %s" % [nick_text, DEPT_SHORT[dept]] if DEPT_SHORT.has(dept) else nick_text


func _refresh_own_label() -> void:
	var official: bool = have_state and (pred.access & MapData.ACCESS_CARD) != 0
	me.set_nick(_label_for(nick, department if official else 0))


func _sample_input(delta: float) -> int:
	if input_blocked or me.status in [Protocol.ACT_RIDING, Protocol.ACT_HELD]:
		return 0
	if not goto_legs.is_empty() or not _goto_path.is_empty():
		var g := _goto_input(delta)  # dev script also drives the computer screen / dialogs
		return 0 if screen.visible or dialog.visible else g
	if dialog.visible:
		return 0
	if screen.visible:
		return 0
	if autowalk:
		_autowalk_timer -= delta
		if _autowalk_timer <= 0.0:
			_autowalk_bits = [0, 1, 2, 4, 8, 5, 9, 6, 10][randi() % 9]
			_autowalk_timer = randf_range(0.3, 1.5)
		return _autowalk_bits
	if not get_window().has_focus():
		return 0
	var b := 0
	if Input.is_physical_key_pressed(KEY_W) or Input.is_physical_key_pressed(KEY_UP):
		b |= Movement.IN_UP
	if Input.is_physical_key_pressed(KEY_S) or Input.is_physical_key_pressed(KEY_DOWN):
		b |= Movement.IN_DOWN
	if Input.is_physical_key_pressed(KEY_A) or Input.is_physical_key_pressed(KEY_LEFT):
		b |= Movement.IN_LEFT
	if Input.is_physical_key_pressed(KEY_D) or Input.is_physical_key_pressed(KEY_RIGHT):
		b |= Movement.IN_RIGHT
	if Input.is_physical_key_pressed(KEY_E):
		b |= Movement.IN_INTERACT
	return b


func _goto_input(delta: float) -> int:
	if goto_delay > 0.0:
		goto_delay -= delta
		return 0
	if _goto_path.is_empty() and not goto_legs.is_empty():
		var leg := goto_legs[0]
		goto_legs.remove_at(0)
		if leg == "E":
			goto_delay = 0.3
			return Movement.IN_INTERACT
		if leg.begins_with("wait:"):
			goto_delay = float(leg.substr(5))
			return 0
		if leg.begins_with("dlg:"):  # dlg:<choice> answers the open dialog
			dialog._choose(int(leg.substr(4)))
			goto_delay = 0.5
			return 0
		if leg.begins_with("shop:"):  # shop:<shelf>:<kind> takes one off a shelf
			net.send(Protocol.encode_shop_take(net.token, int(leg.get_slice(":", 1)), int(leg.get_slice(":", 2))))
			goto_delay = 0.4
			return 0
		if leg == "esc":  # dev: press Esc (the game menu)
			var ev := InputEventKey.new()
			ev.keycode = KEY_ESCAPE
			ev.physical_keycode = KEY_ESCAPE
			ev.pressed = true
			Input.parse_input_event(ev)
			goto_delay = 0.3
			return 0
		if leg == "L":  # lock / unlock the stall
			net.send(Protocol.encode_door_action(net.token))
			goto_delay = 0.3
			return 0
		if leg.begins_with("pc:"):  # computer screen, see ComputerScreen.dev_command
			screen.dev_command(leg.substr(3))
			goto_delay = 0.6
			return 0
		if leg.begins_with("item:"):  # item:take0..2 / put / drop / give / use
			var a := leg.substr(5)
			match a:
				"put": _item_action(Protocol.ITEM_PUT_AWAY, 0)
				"drop": _item_action(Protocol.ITEM_DROP, 0)
				"give": _item_action(Protocol.ITEM_GIVE, 0)
				"use": _item_action(Protocol.ITEM_USE, 0)
				_: _item_action(Protocol.ITEM_TAKE_OUT, int(a.substr(4)))
			goto_delay = 0.4
			return 0
		_goto_path = _plan_path(leg)
		_goto_floor = pred.floor
		goto_delay = 0.3
	if not _goto_path.is_empty() and pred.floor != _goto_floor:
		_goto_path.clear()  # the stairs / elevator took us elsewhere: leg done
		return 0
	while not _goto_path.is_empty():
		var c := Movement.tile_center(_goto_path[0].x, _goto_path[0].y)
		var p: Vector2i = pred.pos
		var b := 0
		if c.x - p.x > Movement.SPEED / 2: b |= Movement.IN_RIGHT
		elif c.x - p.x < -Movement.SPEED / 2: b |= Movement.IN_LEFT
		if c.y - p.y > Movement.SPEED / 2: b |= Movement.IN_DOWN
		elif c.y - p.y < -Movement.SPEED / 2: b |= Movement.IN_UP
		if b != 0:
			return b
		_goto_path.pop_front()
	return 0


## Dev helper; plans on the current floor only.
func _plan_path(leg: String) -> Array[Vector2i]:
	var map = building.get_floor(pred.floor)
	var astar := AStarGrid2D.new()
	astar.region = Rect2i(0, 0, map.width, map.height)
	astar.diagonal_mode = AStarGrid2D.DIAGONAL_MODE_NEVER
	astar.update()
	var goal := Vector2i(-1, -1)
	for y in map.height:
		for x in map.width:
			# Tiles we can't enter (walls, gates without a pass) are solid.
			# Closed doors (elevator, stall) count as passable: walk up and wait.
			if map.is_blocked(x, y) or (map.blocks(x, y, pred.access, MapData.DIR_UP) and not map.is_closed(x, y)):
				astar.set_point_solid(Vector2i(x, y))
			elif goal.x < 0 and map.room_name(map.room_at_tile(x, y)) == leg:
				goal = Vector2i(x + 2, y + 2)  # a bit inside the room
	if leg.contains(","):
		goal = Vector2i(int(leg.get_slice(",", 0)), int(leg.get_slice(",", 1)))
	if goal.x < 0 or map.is_blocked(goal.x, goal.y):
		push_warning("goto: can't find '%s'" % leg)
		return []
	return astar.get_id_path(Movement.tile_of_pos(pred.pos), goal)


func _physics_process(delta: float) -> void:
	if not have_state or not net.is_playing():
		return
	var bits := _sample_input(delta)
	seq += 1
	pending.append([seq, bits])
	if pending.size() > MAX_PENDING:
		pending.pop_front()
	var before_floor: int = pred.floor
	prev_pos = pred.pos
	pred = Movement.step(building, pred, bits)
	if pred.floor != before_floor:
		prev_pos = pred.pos  # changed floors: no lerp across the jump
		_show_floor(pred.floor)
	var d := Movement.input_dir(bits)
	if d.y > 0: me.set_facing(0)
	elif d.y < 0: me.set_facing(1)
	elif d.x < 0: me.set_facing(2)
	elif d.x > 0: me.set_facing(3)
	var k := mini(pending.size(), INPUT_REDUNDANCY)
	var inputs := PackedByteArray()
	for i in range(pending.size() - k, pending.size()):
		inputs.append(pending[i][1])
	net.send(Protocol.encode_input(net.token, latest_tick, seq, inputs))


func _process(delta: float) -> void:
	# Fire alarm: the screen pulses red.
	var pulse := 0.5 + 0.5 * sin(Time.get_ticks_msec() / 1000.0 * TAU * 1.5)
	alarm_tint.color.a = 0.16 * pulse if fire_alarm else 0.0
	alarm_label.modulate.a = 0.55 + 0.45 * pulse
	if have_state:
		error_offset *= exp(-ERROR_DECAY * delta)
		if error_offset.length_squared() < 0.0025:
			error_offset = Vector2.ZERO
		var frac := Engine.get_physics_interpolation_fraction()
		me.position = Movement.to_px(prev_pos).lerp(Movement.to_px(pred.pos), frac) + error_offset
		_update_hint()
		if not _log.is_empty():
			_refresh_log()
	for id in floor_items.keys():
		if latest_tick - floor_items[id].last_seen_tick > REMOTE_TIMEOUT_TICKS:
			floor_items[id].queue_free()
			floor_items.erase(id)
	if shelf_window.visible and have_state and Movement.to_px(pred.pos).distance_to(_shelf_at) > 20.0:
		shelf_window.close()  # walked away from the shelf
	if fridge_window.visible and have_state and Movement.to_px(pred.pos).distance_to(_fridge_at) > 20.0:
		fridge_window.close()
	if have_state:
		_update_stall_doors()
		_update_ride()
		_update_weather()
	for id in trays.keys():
		if latest_tick - trays[id].last_seen_tick > REMOTE_TIMEOUT_TICKS:
			trays[id].queue_free()
			trays.erase(id)
	for id in vehicles.keys():
		if latest_tick - vehicles[id].last_seen_tick > REMOTE_TIMEOUT_TICKS:
			vehicles[id].queue_free()
			vehicles.erase(id)
	for id in computers.keys():
		if latest_tick - computers[id].last_seen_tick > REMOTE_TIMEOUT_TICKS:
			computers[id].queue_free()
			computers.erase(id)
	if have_time:
		est_tick += delta * tick_hz
		var render_tick := est_tick - INTERP_DELAY_SEC * tick_hz
		for id in remotes.keys():
			var r = remotes[id]
			if latest_tick - r.last_seen_tick > REMOTE_TIMEOUT_TICKS:
				r.queue_free()
				remotes.erase(id)
			else:
				var underrun: bool = r.update_render(render_tick)
				# Only players still present in the latest snapshot count; ones
				# that just left the room naturally run out of samples.
				if r.last_seen_tick == latest_tick and r.samples.size() >= 3:
					interp_frames += 1
					if underrun:
						interp_underruns += 1


func _on_packet(p: Dictionary) -> void:
	match p.type:
		Protocol.T_SNAPSHOT:
			_on_snapshot(p)
		Protocol.T_INVENTORY:
			inventory = p.slots
			hud.update_slots(inventory)
			me.set_held(inventory[0].kind if not inventory.is_empty() else 0)
			fridge_window.set_held(inventory[0].kind if not inventory.is_empty() else 0)
		Protocol.T_DOORS:
			var dm = building.get_floor(p.floor)
			if dm:
				dm.set_closed_tiles(p.tiles)
			lift_floor = p.lift_floor
			lift_target = p.lift_target
			lift_moving = p.lift_moving
		Protocol.T_SHELF:
			shelf_window.show_shelf(p)
			_shelf_at = Movement.to_px(pred.pos)
		Protocol.T_FRIDGE:
			if not fridge_window.visible:
				_fridge_at = Movement.to_px(pred.pos)
			fridge_window.show_fridge(p)
		Protocol.T_SMOKE:
			smoke_view.on_smoke(p)
		Protocol.T_LIGHTS:
			light_view.on_lights(p)
		Protocol.T_CLOCK:
			commute_mode = p.mode
			fire_alarm = p.get("alarm", 0) == 1
			smoke_view.alarm = fire_alarm
			alarm_label.visible = fire_alarm
			game_minute = p.minute
			var part := "noc" if p.night else ("rano" if p.minute < 10 * 60 else ("dzień" if p.minute < 18 * 60 else "wieczór"))
			weather = p.weather
			clock_label.text = "Dzień %d · %02d:%02d · %s · %s" % [p.day, p.minute / 60, p.minute % 60, part, Protocol.WEATHER_NAMES.get(weather, "")]
			_update_light()
		Protocol.T_STATS:
			stats_hud.update_stats(p)
			me.set_smelly(p.hygiene < 25)
		Protocol.T_COMPUTER:
			screen.on_computer(p)
		Protocol.T_CALENDAR:
			screen.on_calendar(p)
		Protocol.T_LUNCH_MENU:
			screen.on_lunch(p)
		Protocol.T_COMPANY_OFFERS, Protocol.T_COMPANY_PEOPLE:
			screen.on_company(p)
		Protocol.T_DIALOG:
			dialog.on_dialog(p)
		Protocol.T_CHAT:
			screen.on_chat(p)
		Protocol.T_SOUND:
			sounds.on_sound(p)
		Protocol.T_SAY:
			sounds.on_say(p.id, remotes[p.id].position if remotes.has(p.id) else me.position, p.id == net.player_id)
			var who: String = nicks.get(p.id, "?")
			if p.id == net.player_id:
				who = nick
				me.say(p.text)
			elif remotes.has(p.id):
				remotes[p.id].say(p.text)
			else:
				_pending_say[p.id] = [Time.get_ticks_msec(), p.text]
			_log.append([Time.get_ticks_msec(), "%s: %s" % [who, p.text]])
			if _log.size() > LOG_LINES:
				_log.pop_front()
			_refresh_log()
		Protocol.T_PLAYER_INFO:
			for e in p.players:
				nicks[e.id] = e.nick
				depts[e.id] = e.department
				if kinds.get(e.id, Protocol.KIND_PLAYER) == Protocol.KIND_PLAYER:
					appearances[e.id] = e.appearance
					if remotes.has(e.id):
						remotes[e.id].set_appearance(e.appearance)
				info_requested.erase(e.id)
				if remotes.has(e.id):
					remotes[e.id].set_nick(_label_for(e.nick, e.department))


func _on_snapshot(p: Dictionary) -> void:
	var tick: int = p.tick
	if tick < latest_tick:
		return  # stale / reordered
	if tick > latest_tick:
		latest_tick = tick
		visible_count = 0
		if p.floor != floor_index:
			# Another floor: nobody from the old one is visible any more.
			for r in remotes.values():
				r.queue_free()
			remotes.clear()
			for iv in floor_items.values():
				iv.queue_free()
			floor_items.clear()
			for cv in computers.values():
				cv.queue_free()
			computers.clear()
		elif p.room != room_id and p.frag_cnt == 1:
			# Another room: drop whoever isn't in the new (complete) set right
			# away; people visible from both rooms (e.g. the porter) stay.
			var present := {}
			for e in p.entities:
				present[e.id] = true
			for id in remotes.keys():
				if not present.has(id):
					remotes[id].queue_free()
					remotes.erase(id)
		room_id = p.room
		floor_index = p.floor
		_update_below_view()
		if not have_time or absf(tick - est_tick) > 5.0:
			est_tick = tick
			have_time = true
		else:
			est_tick += (tick - est_tick) * 0.1
		_reconcile(Movement.body(p.floor, Vector2i(p.self_x, p.self_y), p.self_prev_input, p.self_lock, p.self_access, p.self_slow != 0), p.last_input_seq)
		me.set_status(p.self_activity, p.self_slow != 0)
		me.visible = p.self_activity != Protocol.ACT_RIDING  # inside the vehicle
		screen.set_seated(p.self_activity == Protocol.ACT_COMPUTER)
	visible_count += p.entities.size()
	var unknown := []
	var now := Time.get_ticks_msec()
	for e in p.entities:
		if e.kind == Protocol.KIND_ITEM:
			var iv = floor_items.get(e.id)
			if iv == null:
				iv = ItemView.new()
				world.add_child(iv)
				floor_items[e.id] = iv
			iv.setup(e.held)
			iv.position = Vector2(e.x, e.y) / float(Movement.SUBPIXELS)
			iv.last_seen_tick = tick
			continue
		if e.kind == Protocol.KIND_TRAY:
			var tv = trays.get(e.id)
			if tv == null:
				tv = TrayView.new()
				world.add_child(tv)
				trays[e.id] = tv
			tv.set_state(e.held, e.activity)
			tv.position = Vector2(e.x, e.y) / float(Movement.SUBPIXELS)
			tv.last_seen_tick = tick
			kinds[e.id] = e.kind
			continue
		if e.kind == Protocol.KIND_VEHICLE:
			var vv = vehicles.get(e.id)
			if vv == null:
				vv = VehicleView.new()
				world.add_child(vv)
				vehicles[e.id] = vv
			vv.setup(e.held, e.id)
			vv.push(Vector2(e.x, e.y) / float(Movement.SUBPIXELS), e.flags)
			vv.last_seen_tick = tick
			kinds[e.id] = e.kind
			continue
		if e.kind == Protocol.KIND_COMPUTER:
			var cv = computers.get(e.id)
			if cv == null:
				cv = ComputerView.new()
				world.add_child(cv)
				computers[e.id] = cv
			cv.set_flags(e.flags)
			cv.position = Vector2(e.x, e.y) / float(Movement.SUBPIXELS)
			cv.last_seen_tick = tick
			kinds[e.id] = e.kind
			continue
		var r = remotes.get(e.id)
		if r == null:
			r = RemotePlayer.new()
			var npc: bool = e.kind == Protocol.KIND_NPC
			r.look = (e.flags >> 3) & 7 if npc else 0
			r.setup(e.id, _label_for(nicks.get(e.id, "..."), depts.get(e.id, 0)), ZOOM * zoom_level)
			if not npc and appearances.has(e.id):
				r.set_appearance(appearances[e.id])
			world.add_child(r)
			remotes[e.id] = r
			if _pending_say.has(e.id):
				if now - _pending_say[e.id][0] < 4000:
					r.say(_pending_say[e.id][1])
				_pending_say.erase(e.id)
		r.push_sample(tick, Vector2(e.x, e.y) / float(Movement.SUBPIXELS), e.flags)
		r.set_status(e.activity, (e.flags & Protocol.FLAG_SLOW) != 0)
		r.set_smelly((e.flags & Protocol.FLAG_SMELLY) != 0)
		r.set_umbrella(e.kind == Protocol.KIND_PLAYER and (e.flags & Protocol.FLAG_UMBRELLA) != 0)
		r.set_held(e.held)
		kinds[e.id] = e.kind
		if not nicks.has(e.id) and now - info_requested.get(e.id, -100000) > 500:
			info_requested[e.id] = now
			unknown.append(e.id)
	if not unknown.is_empty() and net.is_playing():
		net.send(Protocol.encode_info_request(net.token, unknown))


## Server state (at input `ack`) + replay of the inputs it hasn't seen yet.
func _reconcile(server_body: Dictionary, ack: int) -> void:
	if ack < last_ack:
		return
	last_ack = ack
	while not pending.is_empty() and pending[0][0] <= ack:
		pending.pop_front()
	var nb := server_body
	for inp in pending:
		nb = Movement.step(building, nb, inp[1])
	if not have_state:
		have_state = true
		pred = nb
		prev_pos = nb.pos
		me.position = Movement.to_px(nb.pos)
		me.visible = true
		_show_floor(nb.floor)
		_refresh_own_label()
		entered_world.emit()
		return
	if nb.pos != pred.pos or nb.floor != pred.floor:
		corrections += 1
		if nb.floor != pred.floor:
			error_offset = Vector2.ZERO  # different floor: snap
			prev_pos = nb.pos
			_show_floor(nb.floor)
		else:
			error_offset += Movement.to_px(pred.pos) - Movement.to_px(nb.pos)
			if error_offset.length() > 48.0:
				error_offset = Vector2.ZERO  # large jump: snap
			prev_pos += nb.pos - pred.pos
	var access_changed: bool = nb.access != pred.access
	pred = nb  # also picks up server-side changes (e.g. a new pass)
	if access_changed:
		_refresh_own_label()


## Something in the game takes Esc itself (a window is open).
func window_open() -> bool:
	return screen.visible or shelf_window.visible or fridge_window.visible or dialog.visible


## Settings changed in the Esc menu.
func apply_settings() -> void:
	mood.visible = Settings.mood
	set_zoom_level(Settings.zoom)


## Camera zoom (mouse wheel, + / -): a multiplier of ZOOM.
const ZOOM_MIN := 0.6
const ZOOM_MAX := 2.0
var zoom_level := 1.0


func set_zoom_level(z: float) -> void:
	zoom_level = clampf(z, ZOOM_MIN, ZOOM_MAX)
	var zz := ZOOM * zoom_level
	camera.zoom = Vector2(zz, zz)
	me.set_zoom(zz)
	for r in remotes.values():
		if r.has_method("set_zoom"):
			r.set_zoom(zz)
	for v in views.values():
		v.set_zoom(zz)


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.pressed and not screen.visible and not input_blocked:
		if event.button_index == MOUSE_BUTTON_WHEEL_UP:
			set_zoom_level(zoom_level * 1.1)
		elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			set_zoom_level(zoom_level / 1.1)
	if event is InputEventKey and event.pressed and not input_blocked and not screen.visible:
		if event.physical_keycode in [KEY_EQUAL, KEY_KP_ADD]:
			set_zoom_level(zoom_level * 1.15)
		elif event.physical_keycode in [KEY_MINUS, KEY_KP_SUBTRACT]:
			set_zoom_level(zoom_level / 1.15)
	if not (event is InputEventKey and event.pressed and not event.echo) or input_blocked or screen.visible or not have_state:
		return
	match event.physical_keycode:
		KEY_1, KEY_2, KEY_3:
			_pocket_key(event.physical_keycode - KEY_1)
		KEY_Q:
			_item_action(Protocol.ITEM_DROP, 0)
		KEY_G:
			_item_action(Protocol.ITEM_GIVE, 0)
		KEY_F:
			_item_action(Protocol.ITEM_USE, 0)
		KEY_L:
			if net.is_playing():
				net.send(Protocol.encode_door_action(net.token))


## Pocket key: take it out, or put back what's in hands if that pocket is empty.
func _pocket_key(pocket: int) -> void:
	var slot: Dictionary = inventory[pocket + 1] if pocket + 1 < inventory.size() else {"kind": 0}
	if slot.kind == 0 and me.held in ItemArt.SMALL:
		_item_action(Protocol.ITEM_PUT_AWAY, 0)
	else:
		_item_action(Protocol.ITEM_TAKE_OUT, pocket)


func _computer_action(action: int, conv: int, arg: int, text: String) -> void:
	if net.is_playing():
		net.send(Protocol.encode_computer_action(net.token, action, conv, arg, text))


func _item_action(action: int, slot: int) -> void:
	if net.is_playing():
		net.send(Protocol.encode_item_action(net.token, action, slot))


## Context hint at the bottom of the screen: elevator, NPC to talk to, or a
## gate that needs a pass.
func _update_hint() -> void:
	var text := ""
	if me.status == Protocol.ACT_HELD:
		hint_label.text = "Zatrzymano cię — chwilę stoisz w miejscu…"
		hint_label.visible = true
		return
	var map = building.get_floor(pred.floor)
	var t := Movement.tile_of_pos(pred.pos)
	var link: Dictionary = map.link_at(t.x, t.y) if map else {}
	if not link.is_empty() and link.kind == "elevator":
		# In the cabin: the car stands here -> choose the floor; else riding.
		var standing: bool = not lift_moving and lift_floor == pred.floor
		var target: int = building.next_elevator_floor(pred.floor, link.id)
		if standing and target >= 0:
			text = "[E] Jedź na: %s" % building.floor_name(target)
		else:
			text = "Jedziemy…"
	elif map:
		text = _elevator_call_hint(map)
	if text == "":
		# Same choice as the server: NPCs standing at their post first, then nearest.
		var me_px := Movement.to_px(pred.pos)
		var best_id := -1
		var best_key := Vector2(INF, INF)
		for id in remotes:
			var d: float = remotes[id].position.distance_to(me_px)
			if kinds.get(id) == Protocol.KIND_NPC and d <= TALK_RADIUS_PX:
				var moving: bool = remotes[id].samples.size() > 0 and (remotes[id].samples[-1][2] & 4) != 0
				var key := Vector2(1.0 if moving else 0.0, d)
				if key < best_key:
					best_key = key
					best_id = id
		if best_id >= 0:
			var who: String = nicks.get(best_id, "?")
			text = "[E] Kasa — zapłać za zakupy" if who == "Kasa" else "[E] Porozmawiaj: %s" % who
	if text == "" and map:
		# Kitchenette things (the nearest within 1.5 tiles).
		var kitchen_names := {"cupboard": "[E] Szafka z kubkami", "dishwasher": "[E] Zmywarka", "fridge": "[E] Lodówka", "kitchen_sink": "[E] Zlew"}
		var best_d := INF
		for dy in range(-2, 3):
			for dx in range(-2, 3):
				var kx: int = t.x + dx
				var ky: int = t.y + dy
				if kx < 0 or ky < 0 or kx >= map.width or ky >= map.height:
					continue
				var ktype: String = map.legend.get(map.tile_chars[ky * map.width + kx], {}).get("type", "")
				# The machine, fruit bowl and sanitizer stand in the same row:
				# nearer than a cupboard, they get their own hint (below).
				if not kitchen_names.has(ktype) and not ktype in ["coffee_machine", "fruit_bowl", "sanitizer"]:
					continue
				var kd: float = ((Vector2(kx, ky) + Vector2(0.5, 0.5)) * map.tile_px).distance_to(Movement.to_px(pred.pos))
				if kd <= map.tile_px * 1.5 and kd < best_d:
					best_d = kd
					text = kitchen_names.get(ktype, "")
	if text == "" and map and pred.floor == 0:
		# The way home (like the server): our car / bike, or the spot on foot /
		# at the tram stop / taxi stand.
		var me_p := Movement.to_px(pred.pos)
		var spots := {1: Vector2i(2, 35), 4: Vector2i(34, 36), 5: Vector2i(36, 45)}
		if spots.has(commute_mode):
			var sp: Vector2 = (Vector2(spots[commute_mode]) + Vector2(0.5, 0.5)) * map.tile_px
			if sp.distance_to(me_p) <= map.tile_px * 2:
				text = "[E] Wracam do domu"
		elif commute_mode == 3 or commute_mode == 2:
			var want_kind := 1 if commute_mode == 3 else 2
			for id in vehicles:
				if vehicles[id].kind == want_kind and vehicles[id].position.distance_to(me_p) <= map.tile_px * 2:
					text = "[E] Wracam do domu (%s)" % ("samochodem" if commute_mode == 3 else "rowerem")
					break
	if text == "" and map:
		# Light switch within reach (1 tile, like the server).
		var me_c := Movement.to_px(pred.pos)
		for r in map.room_switch:
			var st: Vector2i = map.room_switch[r]
			var sc: Vector2 = (Vector2(st) + Vector2(0.5, 0.5)) * map.tile_px
			if sc.distance_to(me_c) <= map.tile_px:
				text = "[E] Zgaś światło" if light_view._on.has(r) else "[E] Włącz światło"
				break
	if text == "" and map:
		# Coffee machine within reach (same radius as the server: 1.5 tiles).
		for dy in range(-2, 3):
			for dx in range(-2, 3):
				var tx: int = t.x + dx
				var ty: int = t.y + dy
				if tx < 0 or ty < 0 or tx >= map.width or ty >= map.height:
					continue
				if map.legend.get(map.tile_chars[ty * map.width + tx], {}).get("type") != "coffee_machine":
					continue
				if Movement.to_px(Movement.tile_center(tx, ty)).distance_to(Movement.to_px(pred.pos)) <= 24.0:
					if me.held != 0:
						text = "Najpierw odłóż to, co trzymasz (1–3 / Q)"
					elif me.status == Protocol.ACT_BREWING:
						text = "Parzenie kawy…"
					else:
						text = "[E] Zrób kawę"
	if text == "":
		var me_px4 := Movement.to_px(pred.pos)
		for id in trays:
			if trays[id].position.distance_to(me_px4) <= 24.0:
				text = "[E] Weź: %s (%d szt.)" % [ItemArt.item_name(trays[id].kind), trays[id].pieces]
	if text == "" and map:
		text = _desk_hint(map)
	if text == "" and map:
		text = _spot_hint(map)
	if map:
		text = _stall_hint(map, t, text)
	if text == "":
		var me_px2 := Movement.to_px(pred.pos)
		for id in floor_items:
			if floor_items[id].position.distance_to(me_px2) <= 20.0:
				text = "[E] Podnieś: %s" % ItemArt.item_name(floor_items[id].kind)
				break
	if text == "" and me.held != 0:
		var me_px3 := Movement.to_px(pred.pos)
		for id in remotes:
			if kinds.get(id) == Protocol.KIND_PLAYER and remotes[id].position.distance_to(me_px3) <= 32.0:
				text = "[G] Podaj %s: %s" % [ItemArt.item_name(me.held).to_lower(), nicks.get(id, "?")]
				break
	if text == "" and map:
		for dy in [-1, -2]:
			for dx in [-1, 0, 1]:
				var need: int = map.need_at(t.x + dx, t.y + dy)
				if need != 0 and (pred.access & need) == 0:
					if need & MapData.ACCESS_BOARD:
						text = "Zarząd — wstęp tylko na umówione spotkanie (kalendarz na komputerze)"
					elif need & MapData.ACCESS_GUEST:
						text = "Bramka wymaga przepustki — porozmawiaj z portierem (portiernia)"
					else:
						text = "Wstęp tylko dla obsługi"
	hint_label.text = text
	hint_label.visible = text != ""


## Outdoors: rain / fog on screen, an umbrella if you carry one, a darker
## sky; indoors the lights are on (the weather tints less).
func _update_weather() -> void:
	var m = building.get_floor(pred.floor)
	var outdoors: bool = m != null and m.room_outdoor.has(room_id)
	var wet := weather == Protocol.WEATHER_RAIN or weather == Protocol.WEATHER_STORM
	var carries := false
	for s in inventory:
		carries = carries or s.kind == ItemArt.UMBRELLA
	me.set_umbrella(outdoors and wet and carries and me.status != Protocol.ACT_RIDING)
	if outdoors != weather_fx.outdoors or weather != weather_fx.weather:
		weather_fx.set_state(weather, outdoors)
		_update_light()


func _update_light() -> void:
	var tint := Color(1, 1, 1)
	match weather:
		Protocol.WEATHER_CLOUDY: tint = Color(0.88, 0.88, 0.92)
		Protocol.WEATHER_RAIN: tint = Color(0.74, 0.76, 0.84)
		Protocol.WEATHER_STORM: tint = Color(0.6, 0.62, 0.72)
		Protocol.WEATHER_FOG: tint = Color(0.9, 0.9, 0.92)
	# Brightness is per room now (LightView); keep only a hint of the sky's
	# colour (warm dawn, golden evening) over everything.
	var sky := daylight_color(game_minute) * tint
	daylight.color = Color(1, 1, 1).lerp(sky, 0.35)
	light_view.minute = game_minute
	light_view.weather = weather


## World tint by the time of day: warm dawn, white day, golden evening,
## blue dusk (the office closes at 22:00).
static func daylight_color(minute: int) -> Color:
	var keys := [
		[6 * 60, Color(0.72, 0.68, 0.78)],
		[7 * 60 + 30, Color(1.0, 0.93, 0.85)],
		[9 * 60, Color(1, 1, 1)],
		[17 * 60, Color(1, 1, 1)],
		[19 * 60, Color(1.0, 0.88, 0.74)],
		[21 * 60, Color(0.72, 0.72, 0.9)],
		[22 * 60, Color(0.55, 0.57, 0.78)],
	]
	if minute <= keys[0][0]:
		return keys[0][1]
	for i in range(1, keys.size()):
		if minute <= keys[i][0]:
			var t := float(minute - keys[i - 1][0]) / float(keys[i][0] - keys[i - 1][0])
			return (keys[i - 1][1] as Color).lerp(keys[i][1], t)
	return keys[-1][1]


## Riding the elevator: only the cabin is visible, and it shakes a little.
func _update_ride() -> void:
	var m = building.get_floor(pred.floor)
	var t := Movement.tile_of_pos(pred.pos)
	var link: Dictionary = m.link_at(t.x, t.y) if m else {}
	var riding: bool = lift_moving and not link.is_empty() and link.kind == "elevator"
	if riding:
		var r: Rect2i = link.rect
		ride_mask.show_cabin(Rect2(Vector2(r.position) * m.tile_px, Vector2(r.size) * m.tile_px).grow(2))
		camera.offset = Vector2(randf_range(-0.35, 0.35), randf_range(-0.35, 0.35))
		_set_door_views_visible(false)  # nothing of the floor outside the car
		_set_floor_extras_visible(false)
	elif ride_mask.visible:
		ride_mask.visible = false
		camera.offset = Vector2.ZERO
		_set_door_views_visible(true)
		_set_floor_extras_visible(true)


## On a balcony: the floor below shows through the open air (under this
## floor's picture, a bit darker - it's further away); the server sends
## who is down there, on the same grid.
var _below_floor := -1


func _update_below_view() -> void:
	var m = building.get_floor(floor_index)
	var want := floor_index - 1 if m and floor_index > 0 and m.room_below.has(room_id) else -1
	if want == _below_floor:
		return
	if _below_floor >= 0 and views.has(_below_floor):
		views[_below_floor].visible = false
		views[_below_floor].modulate = Color.WHITE
	_below_floor = want
	if want >= 0 and views.has(want):
		views[want].visible = true
		views[want].modulate = Color(0.78, 0.78, 0.82)


## Smoke, detectors and room names are drawn over the ride mask: hide them
## while riding.
func _set_floor_extras_visible(on: bool) -> void:
	smoke_view.visible = on
	if views.has(pred.floor):
		views[pred.floor].labels.visible = on


func _set_door_views_visible(on: bool) -> void:
	for dv in stall_doors.get(pred.floor, []):
		dv.visible = on
	for ev in elevator_doors.get(pred.floor, []):
		ev.visible = on


## Doors open while somebody stands in them (and aren't locked).
func _update_stall_doors() -> void:
	var m = building.get_floor(pred.floor)
	var people: Array[Vector2] = [me.position]
	for id in remotes:
		people.append(remotes[id].position)
	for dv in stall_doors.get(pred.floor, []):
		var busy := false
		for pos in people:
			if absf(pos.x - dv.position.x) < 12.0 and absf(pos.y - dv.position.y) < 11.0:
				busy = true
				break
		dv.set_state(m.is_closed(dv.tile.x, dv.tile.y), busy)
	for ev in elevator_doors.get(pred.floor, []):
		ev.set_state(m.is_closed(ev.tile.x, ev.tile.y), lift_floor, lift_target)


## In a stall: lock / unlock (L). Outside next to a locked stall: "Zajęte".
func _stall_hint(map, t: Vector2i, text: String) -> String:
	var in_doorway: bool = map.legend.get(map.tile_chars[t.y * map.width + t.x], {}).get("type") == "stall_door"
	if map.room_types.get(room_id, "") == "stall" and not in_doorway:
		var locked := false
		for dv in stall_doors.get(pred.floor, []):
			if map.room_at_tile(dv.tile.x, dv.tile.y) == room_id:
				locked = map.is_closed(dv.tile.x, dv.tile.y)
		var l := "[L] Otwórz kabinę" if locked else "[L] Zamknij kabinę"
		return l if text == "" else "%s   %s" % [text, l]
	if text == "":
		for dv in stall_doors.get(pred.floor, []):
			if map.is_closed(dv.tile.x, dv.tile.y) and absi(dv.tile.x - t.x) <= 1 and absi(dv.tile.y - t.y) <= 1:
				return "Zajęte"
	return text


const SPOT_HINTS := {"shelf": "[E] Zobacz półkę", "sofa": "[E] Usiądź na sofie", "toilet": "[E] Skorzystaj z toalety", "ashtray": "[E] Zapal", "fruit_bowl": "[E] Weź owoc", "sink": "[E] Umyj ręce", "sanitizer": "[E] Zdezynfekuj ręce"}


## Next to the elevator doors (outside the cabin): call it / wait / step in.
func _elevator_call_hint(map) -> String:
	var me_px := Movement.to_px(pred.pos)
	for ev in elevator_doors.get(pred.floor, []):
		if ev.position.distance_to(me_px) <= 24.0:
			if not map.is_closed(ev.tile.x, ev.tile.y):
				return "Winda otwarta — wejdź"
			if lift_target == pred.floor:
				return "Winda jedzie… (%s)" % ElevatorDoorView.floor_label(lift_floor)
			return "[E] Wezwij windę"
	return ""


## Sofa / toilet / ashtray / fruit bowl within reach (1.5 tiles, as the server).
func _spot_hint(map) -> String:
	if me.status in [Protocol.ACT_SOFA, Protocol.ACT_TOILET, Protocol.ACT_SMOKING, Protocol.ACT_WASHING]:
		return "[E] Wstań" if me.status != Protocol.ACT_SMOKING else "[E] Zgaś papierosa"
	var me_px := Movement.to_px(pred.pos)
	var t := Movement.tile_of_pos(pred.pos)
	for dy in range(-2, 3):
		for dx in range(-2, 3):
			var tx: int = t.x + dx
			var ty: int = t.y + dy
			if tx < 0 or ty < 0 or tx >= map.width or ty >= map.height:
				continue
			var type: String = map.legend.get(map.tile_chars[ty * map.width + tx], {}).get("type", "")
			if type == "shelf" and map.room_types.get(map.room_at_tile(tx, ty), "") != "shop":
				continue  # only shop shelves have goods
			if SPOT_HINTS.has(type) and Movement.to_px(Movement.tile_center(tx, ty)).distance_to(me_px) <= 24.0:
				return SPOT_HINTS[type]
	return ""


## Desk within reach (same rule as the server: nearest desk tile, 1.25 tiles).
func _desk_hint(map) -> String:
	var me_px := Movement.to_px(pred.pos)
	var t := Movement.tile_of_pos(pred.pos)
	var best := Vector2i(-1, -1)
	var best_d := 20.0
	for dy in range(-2, 3):
		for dx in range(-2, 3):
			var tx: int = t.x + dx
			var ty: int = t.y + dy
			if tx < 0 or ty < 0 or tx >= map.width or ty >= map.height:
				continue
			if map.legend.get(map.tile_chars[ty * map.width + tx], {}).get("type") != "desk":
				continue
			if map.room_types.get(map.room_at_tile(tx, ty), "") != "department":
				continue
			var d := Movement.to_px(Movement.tile_center(tx, ty)).distance_to(me_px)
			if d <= best_d:
				best_d = d
				best = Vector2i(tx, ty)
	if best.x < 0:
		return ""
	var center := Movement.to_px(Movement.tile_center(best.x, best.y))
	for id in computers:
		if computers[id].position.distance_to(center) < 2.0:
			var f: int = computers[id].flags
			var whose: String = nicks.get(id, "?")
			if f & Protocol.PC_FLAG_IN_USE:
				return "Komputer: %s — ktoś przy nim siedzi" % whose
			return "[E] Komputer: %s%s" % [whose, " (zablokowany)" if f & Protocol.PC_FLAG_LOCKED else ""]
	if me.held == ItemArt.LAPTOP:
		return "[E] Połóż laptop na biurku"
	return ""


func _refresh_log() -> void:
	var now := Time.get_ticks_msec()
	while not _log.is_empty() and now - _log[0][0] > LOG_TTL_SEC * 1000:
		_log.pop_front()
	var lines := PackedStringArray()
	for l in _log:
		lines.append(l[1])
	log_label.text = "\n".join(lines)


func debug_text() -> String:
	var t := Movement.tile_of_pos(pred.pos) if have_state else Vector2i.ZERO
	return "\n".join([
		"FPS: %d" % Engine.get_frames_per_second(),
		"Ping: %.0f ms" % net.rtt_ms,
		"Tick serwera: %d  (render %.1f)" % [latest_tick, est_tick - INTERP_DELAY_SEC * tick_hz],
		"Piętro: %s  Pokój: %s (id %d)" % [building.floor_name(floor_index), building.room_name(floor_index, room_id), room_id],
		"Widoczni gracze: %d" % visible_count,
		"Gracz #%d %s  kafel (%d, %d)" % [net.player_id, nick, t.x, t.y],
		"Uprawnienia: %s" % _access_text(),
		"Stanowisko: %s" % (("%s (dział %s), umowa %s" % [job_title, DEPT_NAMES.get(department, "?"), "podpisana" if have_state and (pred.access & MapData.ACCESS_CARD) else "jeszcze nie"]) if department else "-"),
		"Inputy w locie: %d  korekty: %d" % [pending.size(), corrections],
		"Bufor interpolacji pusty: %.2f%% klatek" % (100.0 * interp_underruns / maxi(interp_frames, 1)),
		"Ruch: %.1f KB/s in / %.1f KB/s out" % [net.bytes_in_per_sec / 1024.0, net.bytes_out_per_sec / 1024.0],
		"Serwer: %s  zmiany gniazda: %d  ponowne połączenia: %d" % [net.server_ip, net.rebinds, net.reconnects],
	])


func _access_text() -> String:
	if not have_state:
		return "-"
	var parts := PackedStringArray()
	if pred.access & MapData.ACCESS_GUEST:
		parts.append("przepustka gościa")
	if pred.access & MapData.ACCESS_CARD:
		parts.append("karta pracownika")
	if pred.access & MapData.ACCESS_SERVICE:
		parts.append("obsługa")
	return ", ".join(parts) if not parts.is_empty() else "brak"
