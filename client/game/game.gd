## In-game world: local prediction + reconciliation, remote interpolation,
## room-based visibility, camera and debug info.
extends Node2D

const Protocol = preload("res://net/protocol.gd")
const Movement = preload("res://sim/movement.gd")
const MapView = preload("res://map/map_view.gd")
const PlayerView = preload("res://game/player_view.gd")
const RemotePlayer = preload("res://game/remote_player.gd")
const DebugOverlay = preload("res://ui/debug_overlay.gd")

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

var net
var map
var tick_hz := 20
var nick := ""
var world := Node2D.new()
var me := PlayerView.new()
var camera := Camera2D.new()
var overlay := DebugOverlay.new()
var status_layer := CanvasLayer.new()
var status_label := Label.new()
var remotes := {}        # id -> RemotePlayer
var nicks := {}          # id -> String
var info_requested := {} # id -> msec of last request

# Local prediction state (sub-pixel units).
var have_state := false
var pred := Vector2i.ZERO
var prev_pred := Vector2i.ZERO
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

# Dev helpers: --autowalk (random walk), --goto=<room name> (walk to a room).
var autowalk := false
var _autowalk_bits := 0
var _autowalk_timer := 0.0
var goto_room := ""
var goto_delay := 3.0
var _goto_path: Array[Vector2i] = []


func setup(p_net, p_map, welcome: Dictionary, p_nick: String, args: Dictionary) -> void:
	net = p_net
	map = p_map
	nick = p_nick
	tick_hz = welcome.tick_hz
	autowalk = args.has("autowalk")
	goto_room = args.get("goto", "")
	goto_delay = float(args.get("goto-delay", "3"))
	net.packet_received.connect(_on_packet)

	var view := MapView.new()
	view.build(map, ZOOM)
	add_child(view)
	world.y_sort_enabled = true
	add_child(world)

	me.setup(_color_for(net.player_id), nick, ZOOM)
	me.outline = Color.WHITE
	me.visible = false
	world.add_child(me)
	camera.zoom = Vector2(ZOOM, ZOOM)
	camera.limit_left = 0
	camera.limit_top = 0
	camera.limit_right = map.width * map.tile_px
	camera.limit_bottom = map.height * map.tile_px
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
	if goto_room != "":
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
	return b


func _goto_input(delta: float) -> int:
	if goto_delay > 0.0:
		goto_delay -= delta
		if goto_delay <= 0.0:
			_goto_path = _plan_path_to_room(goto_room)
		return 0
	while not _goto_path.is_empty():
		var c := Movement.tile_center(_goto_path[0].x, _goto_path[0].y)
		var b := 0
		if c.x - pred.x > Movement.SPEED / 2: b |= Movement.IN_RIGHT
		elif c.x - pred.x < -Movement.SPEED / 2: b |= Movement.IN_LEFT
		if c.y - pred.y > Movement.SPEED / 2: b |= Movement.IN_DOWN
		elif c.y - pred.y < -Movement.SPEED / 2: b |= Movement.IN_UP
		if b != 0:
			return b
		_goto_path.pop_front()
	return 0


func _plan_path_to_room(room_name: String) -> Array[Vector2i]:
	var astar := AStarGrid2D.new()
	astar.region = Rect2i(0, 0, map.width, map.height)
	astar.diagonal_mode = AStarGrid2D.DIAGONAL_MODE_NEVER
	astar.update()
	var goal := Vector2i(-1, -1)
	for y in map.height:
		for x in map.width:
			if map.is_blocked(x, y):
				astar.set_point_solid(Vector2i(x, y))
			elif goal.x < 0 and map.room_name(map.room_at_tile(x, y)) == room_name:
				goal = Vector2i(x + 2, y + 2)  # a bit inside the room
	if goal.x < 0 or map.is_blocked(goal.x, goal.y):
		push_warning("goto: no room '%s'" % room_name)
		return []
	var from := Vector2i(pred.x / Movement.TILE_UNITS, pred.y / Movement.TILE_UNITS)
	return astar.get_id_path(from, goal)


func _physics_process(delta: float) -> void:
	if not have_state or not net.is_playing():
		return
	var bits := _sample_input(delta)
	seq += 1
	pending.append([seq, bits])
	if pending.size() > MAX_PENDING:
		pending.pop_front()
	prev_pred = pred
	pred = Movement.step(map, pred, bits)
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
		me.position = Movement.to_px(prev_pred).lerp(Movement.to_px(pred), frac) + error_offset
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
		_reconcile(Vector2i(p.self_x, p.self_y), p.last_input_seq)
	visible_count += p.entities.size()
	var unknown := []
	var now := Time.get_ticks_msec()
	for e in p.entities:
		var r = remotes.get(e.id)
		if r == null:
			r = RemotePlayer.new()
			r.setup(_color_for(e.id), nicks.get(e.id, "..."), ZOOM)
			world.add_child(r)
			remotes[e.id] = r
		r.push_sample(tick, Vector2(e.x, e.y) / float(Movement.SUBPIXELS), e.flags)
		if not nicks.has(e.id) and now - info_requested.get(e.id, -100000) > 500:
			info_requested[e.id] = now
			unknown.append(e.id)
	if not unknown.is_empty() and net.is_playing():
		net.send(Protocol.encode_info_request(net.token, unknown))


func _reconcile(server_pos: Vector2i, ack: int) -> void:
	if ack < last_ack:
		return
	last_ack = ack
	while not pending.is_empty() and pending[0][0] <= ack:
		pending.pop_front()
	var np := server_pos
	for inp in pending:
		np = Movement.step(map, np, inp[1])
	if not have_state:
		have_state = true
		pred = np
		prev_pred = np
		me.position = Movement.to_px(np)
		me.visible = true
		return
	if np != pred:
		corrections += 1
		error_offset += Movement.to_px(pred) - Movement.to_px(np)
		if error_offset.length() > 48.0:
			error_offset = Vector2.ZERO  # large jump (e.g. teleport): snap
		prev_pred += np - pred
		pred = np


func debug_text() -> String:
	var tx: int = pred.x / Movement.TILE_UNITS
	var ty: int = pred.y / Movement.TILE_UNITS
	return "\n".join([
		"FPS: %d" % Engine.get_frames_per_second(),
		"Ping: %.0f ms" % net.rtt_ms,
		"Tick serwera: %d  (render %.1f)" % [latest_tick, est_tick - INTERP_DELAY_SEC * tick_hz],
		"Pokój: %s (id %d, piętro %d)" % [map.room_name(room_id), room_id, floor_index],
		"Widoczni gracze: %d" % visible_count,
		"Gracz #%d %s  kafel (%d, %d)" % [net.player_id, nick, tx, ty],
		"Inputy w locie: %d  korekty: %d" % [pending.size(), corrections],
		"Bufor interpolacji pusty: %.2f%% klatek" % (100.0 * interp_underruns / maxi(interp_frames, 1)),
		"Ruch: %.1f KB/s in / %.1f KB/s out" % [net.bytes_in_per_sec / 1024.0, net.bytes_out_per_sec / 1024.0],
		"Serwer: %s  zmiany gniazda: %d  ponowne połączenia: %d" % [net.server_ip, net.rebinds, net.reconnects],
	])
