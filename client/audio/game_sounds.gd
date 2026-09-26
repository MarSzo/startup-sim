## Sounds of the world while playing: footsteps (by surface), the server's
## Sound events, speech blips, ambience beds (street, office hum, rain, the
## fire bell) and sirens on police cars / fire engines.
extends Node

const Audio = preload("res://audio/audio.gd")
const Protocol = preload("res://net/protocol.gd")
const Movement = preload("res://sim/movement.gd")

## A step every this many px walked.
const STEP_PX := 20.0
## Remote steps are heard this close (px).
const STEP_HEAR_PX := 260.0
const VEHICLE_POLICE := 5
const VEHICLE_FIRE := 6

var game
var _walked := {}      # id (0 = me) -> [last px position, px since the last step]
var _step_n := 0
var _last_blip := {}   # speaker -> msec
var _open := {}        # window -> was visible


func _process(_delta: float) -> void:
	var a = Audio.inst
	if a == null or game == null:
		return
	if not game.have_state or game.input_blocked and not game.me.visible:
		for s in ["street_loop", "office_loop", "rain_loop", "fire_bell_loop"]:
			a.bed(s, -80.0)
		a.prune_loops({})
		return
	# A window (fridge, shelf, dialog, computer) opens: a paper swish.
	for w in [game.fridge_window, game.shelf_window, game.dialog, game.screen]:
		if w.visible and not _open.get(w, false):
			a.play("ui_open", -8.0, 0.1)
		_open[w] = w.visible
	var map = game.building.get_floor(game.pred.floor)
	var outdoors: bool = map != null and map.room_outdoor.has(game.room_id)
	# Steps: ours, then the people around.
	if game.me.visible:
		_step(0, game.me.position, map, -7.0)
	var seen := {0: true}
	for id in game.remotes:
		var r = game.remotes[id]
		if r.visible and r.position.distance_to(game.me.position) < STEP_HEAR_PX:
			_step(id, r.position, map, -13.0)
			seen[id] = true
	for id in _walked.keys():
		if not seen.has(id):
			_walked.erase(id)
	# Ambience.
	var floor0: bool = game.pred.floor == 0
	a.bed("street_loop", -8.0 if outdoors else (-26.0 if floor0 else -34.0))
	a.bed("office_loop", -80.0 if outdoors else -16.0)
	var wet: bool = game.weather in [Protocol.WEATHER_RAIN, Protocol.WEATHER_STORM]
	var rain_db := (-3.0 if game.weather == Protocol.WEATHER_STORM else -6.0) if outdoors else -24.0
	a.bed("rain_loop", rain_db if wet else -80.0)
	a.bed("fire_bell_loop", (-18.0 if outdoors else -6.0) if game.fire_alarm else -80.0)
	# Sirens on the emergency vehicles in sight.
	var keep := {}
	for id in game.vehicles:
		var v = game.vehicles[id]
		if v.visible and v.kind in [VEHICLE_POLICE, VEHICLE_FIRE]:
			var file := "siren_police_loop" if v.kind == VEHICLE_POLICE else "siren_fire_loop"
			a.loop_at(id, file, v.position, true, -6.0)
			keep[id] = true
	a.prune_loops(keep)


func _step(id: int, pos: Vector2, map, volume_db: float) -> void:
	if not _walked.has(id):
		_walked[id] = [pos, 0.0]
		return
	var w: Array = _walked[id]
	var d: float = pos.distance_to(w[0])
	w[0] = pos
	if d > 40.0:  # a jump (stairs, elevator, a correction): no step
		return
	w[1] += d
	if w[1] < STEP_PX:
		return
	w[1] = 0.0
	_step_n = _step_n % 3 + 1
	var file := "step_%s_%d" % [_surface(map, pos), _step_n]
	if id == 0:
		Audio.inst.play(file, volume_db, 0.08)
	else:
		Audio.inst.play_at(file, pos, volume_db, 0.08)


## What the floor under `pos` sounds like.
func _surface(map, pos: Vector2) -> String:
	if map == null:
		return "floor"
	var t := Vector2i(floori(pos.x / map.tile_px), floori(pos.y / map.tile_px))
	if t.x < 0 or t.y < 0 or t.x >= map.width or t.y >= map.height:
		return "floor"
	if map.room_outdoor.has(map.room_at_tile(t.x, t.y)):
		return "out"
	match map.legend.get(map.tile_chars[t.y * map.width + t.x], {}).get("type", ""):
		"carpet":
			return "carpet"
		"tiles", "balcony":
			return "tiles"
	return "floor"


## Sound packet: things happening around us.
func on_sound(p: Dictionary) -> void:
	if Audio.inst == null:
		return
	for s in p.sounds:
		var file: String = Protocol.SOUND_FILES.get(s[0], "")
		if file != "":
			Audio.inst.play_at(file, Movement.to_px(Vector2i(s[1], s[2])), -2.0, 0.05)


## Someone said something: a short voice blip (pitch per speaker).
func on_say(id: int, pos: Vector2, mine: bool) -> void:
	if Audio.inst == null:
		return
	var now := Time.get_ticks_msec()
	if now - _last_blip.get(id, 0) < 250:
		return
	_last_blip[id] = now
	var pitch := 0.8 + float(id % 7) * 0.07
	if mine:
		Audio.inst.play("blip", -12.0, 0.03, pitch)
	else:
		Audio.inst.play_at("blip", pos, -8.0, 0.03, pitch)


## Lightning (weather_fx): thunder a moment later.
func on_lightning(outdoors: bool) -> void:
	if Audio.inst == null:
		return
	await get_tree().create_timer(randf_range(0.3, 1.2)).timeout
	Audio.inst.play("thunder", -2.0 if outdoors else -12.0, 0.1)
