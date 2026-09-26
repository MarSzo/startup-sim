## All the game's sound (files from tools/sounds/gen_sounds.py): one-shot
## effects (flat or placed in the world), ambience beds that fade in and
## out, positional loops (sirens) and music. Three buses — SFX, Ambient,
## Music — with volumes from the settings.
extends Node

const DIR := "res://sounds/"
const BUSES := ["SFX", "Ambient", "Music"]
## How far a placed sound carries (px, world space).
const HEAR_PX := 420.0
const POOL := 20

## The one instance (created by main.gd); null in tests.
static var inst = null

var world: Node2D   # where placed sounds live (the camera's world)
var _streams := {}   # name -> AudioStream
var _flat: Array[AudioStreamPlayer] = []
var _placed: Array[AudioStreamPlayer2D] = []
var _beds := {}      # name -> [player, target_db]
var _loops := {}     # key -> AudioStreamPlayer2D
var _music: Array[AudioStreamPlayer] = []
var _music_name := ""


func _ready() -> void:
	inst = self
	process_mode = Node.PROCESS_MODE_ALWAYS
	for b in BUSES:
		if AudioServer.get_bus_index(b) < 0:
			AudioServer.add_bus()
			var i := AudioServer.bus_count - 1
			AudioServer.set_bus_name(i, b)
			AudioServer.set_bus_send(i, "Master")
	for i in 8:
		var p := AudioStreamPlayer.new()
		p.bus = "SFX"
		add_child(p)
		_flat.append(p)
	for i in 2:
		var m := AudioStreamPlayer.new()
		m.bus = "Music"
		m.volume_db = -80.0
		add_child(m)
		_music.append(m)


## Bus volumes (0..1 each).
func set_volumes(sfx: float, ambient: float, music: float) -> void:
	for pair in [["SFX", sfx], ["Ambient", ambient], ["Music", music]]:
		var i := AudioServer.get_bus_index(pair[0])
		AudioServer.set_bus_volume_db(i, linear_to_db(maxf(pair[1], 0.0001)))
		AudioServer.set_bus_mute(i, pair[1] <= 0.001)


func stream(sound: String, looped := false) -> AudioStream:
	var key := sound + ("@loop" if looped else "")
	if not _streams.has(key):
		var path := DIR + sound + ".wav"
		if not ResourceLoader.exists(path):
			_streams[key] = null
		else:
			var s: AudioStreamWAV = load(path)
			if looped:
				s = s.duplicate()
				s.loop_mode = AudioStreamWAV.LOOP_FORWARD
				s.loop_begin = 0
				s.loop_end = int(s.get_length() * s.mix_rate)
			_streams[key] = s
	return _streams[key]


## A sound for us only (UI, our own steps): not placed.
func play(sound: String, volume_db := 0.0, pitch_var := 0.0, pitch := 1.0) -> void:
	var s := stream(sound)
	if s == null:
		return
	var p: AudioStreamPlayer = _flat[0]
	for c in _flat:
		if not c.playing:
			p = c
			break
	_flat.erase(p)
	_flat.append(p)  # the oldest is reused first
	p.stream = s
	p.volume_db = volume_db
	p.pitch_scale = pitch * (1.0 + randf_range(-pitch_var, pitch_var))
	p.play()


## A sound at a spot in the world (heard fainter further from the camera).
func play_at(sound: String, pos: Vector2, volume_db := 0.0, pitch_var := 0.0, pitch := 1.0) -> void:
	var s := stream(sound)
	if s == null or world == null:
		return
	var p: AudioStreamPlayer2D = null
	for c in _placed:
		if not c.playing:
			p = c
			break
	if p == null:
		if _placed.size() < POOL:
			p = _new_placed()
		else:
			p = _placed.pop_front()
			_placed.append(p)
	p.stream = s
	p.global_position = pos
	p.volume_db = volume_db
	p.pitch_scale = pitch * (1.0 + randf_range(-pitch_var, pitch_var))
	p.play()


func _new_placed() -> AudioStreamPlayer2D:
	var p := AudioStreamPlayer2D.new()
	p.bus = "SFX"
	p.max_distance = HEAR_PX
	p.attenuation = 1.6
	world.add_child(p)
	_placed.append(p)
	return p


## An ambience bed: fades to `volume_db` (or out, with -80).
func bed(sound: String, volume_db: float) -> void:
	if not _beds.has(sound):
		if volume_db <= -79.0:
			return
		var p := AudioStreamPlayer.new()
		p.bus = "Ambient"
		p.stream = stream(sound, true)
		p.volume_db = -60.0
		add_child(p)
		p.play()
		_beds[sound] = [p, volume_db]
	_beds[sound][1] = volume_db


## A loop stuck to something in the world (a siren on a vehicle).
func loop_at(key, sound: String, pos: Vector2, on: bool, volume_db := 0.0) -> void:
	if not on:
		if _loops.has(key):
			_loops[key].queue_free()
			_loops.erase(key)
		return
	if world == null:
		return
	if not _loops.has(key):
		var p := AudioStreamPlayer2D.new()
		p.bus = "SFX"
		p.max_distance = HEAR_PX * 1.6
		p.stream = stream(sound, true)
		world.add_child(p)
		p.play()
		_loops[key] = p
	_loops[key].global_position = pos
	_loops[key].volume_db = volume_db


## Stop the loops whose keys aren't in `keep`.
func prune_loops(keep: Dictionary) -> void:
	for k in _loops.keys():
		if not keep.has(k):
			loop_at(k, "", Vector2.ZERO, false)


## The world went away (back to the menu): drop what hangs on it.
func leave_world() -> void:
	for k in _loops.keys():
		loop_at(k, "", Vector2.ZERO, false)
	for p in _placed:
		p.queue_free()
	_placed.clear()
	world = null
	for s in _beds:
		_beds[s][1] = -80.0


## Music: crossfades to `name` ("" = silence).
func music(name: String) -> void:
	if name == _music_name:
		return
	_music_name = name
	var old: AudioStreamPlayer = _music[0]
	var new: AudioStreamPlayer = _music[1]
	_music.reverse()
	if name != "":
		new.stream = stream(name, true)
		new.volume_db = -40.0
		new.play()
	old.set_meta("fading", true)
	new.set_meta("fading", name == "")


func _process(delta: float) -> void:
	for s in _beds.keys():
		var p: AudioStreamPlayer = _beds[s][0]
		p.volume_db = move_toward(p.volume_db, _beds[s][1], delta * 30.0)
		if p.volume_db <= -79.0 and _beds[s][1] <= -79.0:
			p.queue_free()
			_beds.erase(s)
	for m in _music:
		if not m.playing:
			continue
		var target := -80.0 if m.get_meta("fading", false) else -6.0
		m.volume_db = move_toward(m.volume_db, target, delta * 25.0)
		if m.volume_db <= -79.0:
			m.stop()
