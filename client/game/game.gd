## In-game world: local prediction + reconciliation, remote interpolation,
## floors and room-based visibility, camera and debug info.
extends Node2D

const Protocol = preload("res://net/protocol.gd")
const Movement = preload("res://sim/movement.gd")
const MapView = preload("res://map/map_view.gd")
const PlayerView = preload("res://game/player_view.gd")
const RemotePlayer = preload("res://game/remote_player.gd")
const DebugOverlay = preload("res://ui/debug_overlay.gd")
const MapData = preload("res://map/map_data.gd")

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
const NPC_COLORS := {1: Color(0.22, 0.32, 0.62), 2: Color(0.55, 0.3, 0.45)}  # by look
const LOG_LINES := 4
const LOG_TTL_SEC := 12.0

var net
var building
var views := {}          # floor -> MapView (only the current floor is visible)
var tick_hz := 20
var nick := ""
var world := Node2D.new()
var me := PlayerView.new()
var camera := Camera2D.new()
var overlay := DebugOverlay.new()
var status_layer := CanvasLayer.new()
var status_label := Label.new()
var hint_label := Label.new()
var log_label := Label.new()
var _log: Array = []  # [msec, text]
var kinds := {}          # id -> entity kind (player / NPC)
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


func setup(p_net, p_building, welcome: Dictionary, p_nick: String, args: Dictionary) -> void:
	net = p_net
	building = p_building
	nick = p_nick
	tick_hz = welcome.tick_hz
	autowalk = args.has("autowalk")
	if args.get("goto", "") != "":
		goto_legs = args["goto"].split(";")
	goto_delay = float(args.get("goto-delay", "3"))
	net.packet_received.connect(_on_packet)

	var map0 = building.get_floor(0)
	for f in building.floors.size():
		var m = building.get_floor(f)
		if m == null:
			continue
		var view := MapView.new()
		view.build(m, ZOOM)
		view.visible = false
		add_child(view)
		views[f] = view
	world.y_sort_enabled = true
	add_child(world)

	me.setup(_color_for(net.player_id), nick, ZOOM)
	me.outline = Color.WHITE
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
	status_label.add_theme_font_size_override("font_size", 22)
	status_label.add_theme_constant_override("outline_size", 6)
	status_label.add_theme_color_override("font_outline_color", Color.BLACK)
	status_label.visible = false
	status_layer.add_child(status_label)
	hint_label.set_anchors_preset(Control.PRESET_CENTER_BOTTOM)
	hint_label.position = Vector2(-250, -64)
	hint_label.size = Vector2(500, 36)
	hint_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	hint_label.add_theme_font_size_override("font_size", 20)
	hint_label.add_theme_constant_override("outline_size", 6)
	hint_label.add_theme_color_override("font_outline_color", Color.BLACK)
	hint_label.visible = false
	status_layer.add_child(hint_label)
	log_label.set_anchors_preset(Control.PRESET_BOTTOM_LEFT)
	log_label.position = Vector2(16, -140)
	log_label.size = Vector2(700, 124)
	log_label.vertical_alignment = VERTICAL_ALIGNMENT_BOTTOM
	log_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	log_label.add_theme_font_size_override("font_size", 16)
	log_label.add_theme_constant_override("outline_size", 5)
	log_label.add_theme_color_override("font_outline_color", Color.BLACK)
	status_layer.add_child(log_label)
	_show_floor(0)


func _show_floor(f: int) -> void:
	for k in views:
		views[k].visible = (k == f)


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
	me.color = _color_for(net.player_id)
	me.queue_redraw()
	status_label.visible = false


func _color_for(id: int) -> Color:
	return Color.from_hsv(fmod(id * 0.618034, 1.0), 0.55, 0.95)


func _sample_input(delta: float) -> int:
	if not goto_legs.is_empty() or not _goto_path.is_empty():
		return _goto_input(delta)
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
		_goto_path = _plan_path(leg)
		goto_delay = 0.3
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
			if map.is_blocked(x, y) or map.blocks(x, y, pred.access, MapData.DIR_UP):
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
	if have_state:
		error_offset *= exp(-ERROR_DECAY * delta)
		if error_offset.length_squared() < 0.0025:
			error_offset = Vector2.ZERO
		var frac := Engine.get_physics_interpolation_fraction()
		me.position = Movement.to_px(prev_pos).lerp(Movement.to_px(pred.pos), frac) + error_offset
		_update_hint()
		if not _log.is_empty():
			_refresh_log()
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
		Protocol.T_SAY:
			var who: String = nicks.get(p.id, "?")
			if remotes.has(p.id):
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
				info_requested.erase(e.id)
				if remotes.has(e.id):
					remotes[e.id].set_nick(e.nick)


func _on_snapshot(p: Dictionary) -> void:
	var tick: int = p.tick
	if tick < latest_tick:
		return  # stale / reordered
	if tick > latest_tick:
		latest_tick = tick
		visible_count = 0
		if p.room != room_id or p.floor != floor_index:
			# Entered another room: the visible set is replaced wholesale.
			for r in remotes.values():
				r.queue_free()
			remotes.clear()
			room_id = p.room
			floor_index = p.floor
		if not have_time or absf(tick - est_tick) > 5.0:
			est_tick = tick
			have_time = true
		else:
			est_tick += (tick - est_tick) * 0.1
		_reconcile(Movement.body(p.floor, Vector2i(p.self_x, p.self_y), p.self_prev_input, p.self_lock, p.self_access), p.last_input_seq)
	visible_count += p.entities.size()
	var unknown := []
	var now := Time.get_ticks_msec()
	for e in p.entities:
		var r = remotes.get(e.id)
		if r == null:
			r = RemotePlayer.new()
			var npc: bool = e.kind == Protocol.KIND_NPC
			r.look = (e.flags >> 3) & 7 if npc else 0
			r.setup(NPC_COLORS.get(r.look, Color.GRAY) if npc else _color_for(e.id), nicks.get(e.id, "..."), ZOOM)
			world.add_child(r)
			remotes[e.id] = r
			if _pending_say.has(e.id):
				if now - _pending_say[e.id][0] < 4000:
					r.say(_pending_say[e.id][1])
				_pending_say.erase(e.id)
		r.push_sample(tick, Vector2(e.x, e.y) / float(Movement.SUBPIXELS), e.flags)
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
	pred = nb  # also picks up server-side changes (e.g. a new pass)


## Context hint at the bottom of the screen: elevator, NPC to talk to, or a
## gate that needs a pass.
func _update_hint() -> void:
	var text := ""
	var map = building.get_floor(pred.floor)
	var t := Movement.tile_of_pos(pred.pos)
	var link: Dictionary = map.link_at(t.x, t.y) if map else {}
	if not link.is_empty() and link.kind == "elevator":
		var target: int = building.next_elevator_floor(pred.floor, link.id)
		if target >= 0:
			text = "[E] Winda: jedź na %s" % building.floor_name(target)
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
			text = "[E] Porozmawiaj: %s" % nicks.get(best_id, "?")
	if text == "" and map:
		for dy in [-1, -2]:
			for dx in [-1, 0, 1]:
				var need: int = map.need_at(t.x + dx, t.y + dy)
				if need != 0 and (pred.access & need) == 0:
					text = "Bramka wymaga przepustki — porozmawiaj z portierem (portiernia)" if (need & MapData.ACCESS_GUEST) else "Wstęp tylko dla obsługi"
	hint_label.text = text
	hint_label.visible = text != ""


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
