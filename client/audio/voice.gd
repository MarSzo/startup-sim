## Voice chat (push-to-talk): hold V to talk to the room, B to whisper to
## the person right next to you. The microphone is opened on the first
## press; frames are 40 ms of 16 kHz mono, IMA ADPCM (audio/adpcm.gd). Heard
## voices play from the speaker's character (quieter further away) or, for
## a whisper, straight in the ears. The server decides who hears what.
extends Node

const Adpcm = preload("res://audio/adpcm.gd")
const Protocol = preload("res://net/protocol.gd")

const RATE := 16000
const FRAME := 640                # samples per frame (40 ms)
const TALK_KEY := KEY_V
const WHISPER_KEY := KEY_B
## Client-side guess of the whisper reach (server: 1.5 tiles).
const WHISPER_PX := 24.0
## How long the "talking" waves stay after the last frame.
const TALK_SHOW_MSEC := 300
const HEAR_PX := 520.0

signal send(seq: int, whisper: bool, data: PackedByteArray)

var game
var talking := 0                  # 0 no, 1 room, 2 whisper
var whisper_to := -1              # our guess of whom a whisper reaches
var _mic: AudioStreamPlayer
var _capture: AudioEffectCapture
var _resample_pos := 0.0
var _pending := PackedFloat32Array()
var _enc_state := [0, 0]
var _seq := 0
var _players := {}                # speaker -> {node, playback, whisper}
var _heard := {}                  # speaker -> [msec, whisper]
var _drunk := {}                  # speaker -> drunk tier (entity flags)
var _last_self := 0
## Dev (goto talk:N / whisper:N, --voice-tone): talk without the keys, with
## a test tone instead of the microphone.
var dev_tone := false
var _dev_kind := 0
var _dev_until := 0
var _tone_t := 0.0
var _tone_msec := 0


func _ready() -> void:
	for b in ["Voice", "Mic"]:
		if AudioServer.get_bus_index(b) < 0:
			AudioServer.add_bus()
			var i := AudioServer.bus_count - 1
			AudioServer.set_bus_name(i, b)
			AudioServer.set_bus_send(i, "Master")
	var mic_bus := AudioServer.get_bus_index("Mic")
	if AudioServer.get_bus_effect_count(mic_bus) == 0:
		AudioServer.add_bus_effect(mic_bus, AudioEffectCapture.new())
	_capture = AudioServer.get_bus_effect(mic_bus, 0)
	AudioServer.set_bus_mute(mic_bus, true)  # don't hear yourself
	preload("res://ui/settings.gd").apply_audio()  # the Voice bus volume, the microphone


func _exit_tree() -> void:
	if _mic:
		_mic.stop()


## The microphone starts on the first push-to-talk (macOS asks then).
func _open_mic() -> void:
	if _mic:
		return
	_mic = AudioStreamPlayer.new()
	_mic.stream = AudioStreamMicrophone.new()
	_mic.bus = "Mic"
	add_child(_mic)
	_mic.play()


func _process(_delta: float) -> void:
	var now := Time.get_ticks_msec()
	_update_talking()
	if talking != 0 and dev_tone:
		_read_tone()
	elif talking != 0 and _mic:
		_read_mic()
	elif _capture:
		_capture.clear_buffer()
		_pending.clear()
	# Drunk speakers: the voice wobbles up and down (pitch averages 1.0, so
	# the stream is consumed as fast as it arrives).
	for id in _players:
		var node = _players[id].node
		if is_instance_valid(node):
			var tier: int = _drunk.get(id, 0)
			var depth := 0.0 if tier < 2 else (0.06 if tier == 2 else 0.12)
			node.pitch_scale = 1.0 + sin(now / 1000.0 * 2.3 + id) * depth
	# Waves over the heads of people talking.
	for id in _heard.keys():
		var v = _view(id)
		var on: bool = now - _heard[id][0] < TALK_SHOW_MSEC
		if v:
			v.set_talking(on, _heard[id][1])
		if not on:
			_heard.erase(id)
	if game and game.me:
		game.me.set_talking(now - _last_self < TALK_SHOW_MSEC and talking != 0, talking == 2)


func _update_talking() -> void:
	var can: bool = game != null and game.have_state and not game.input_blocked and game.me.visible
	var focus := get_viewport().gui_get_focus_owner()
	if focus is LineEdit or focus is TextEdit:
		can = false  # typing (the computer's messenger, forms)
	var want := 0
	if can and Time.get_ticks_msec() < _dev_until:
		want = _dev_kind
	elif can and Input.is_key_pressed(WHISPER_KEY):
		want = 2
	elif can and Input.is_key_pressed(TALK_KEY):
		want = 1
	whisper_to = _nearest() if can else -1
	if want != 0 and talking == 0:
		if not dev_tone:
			_open_mic()
		_tone_msec = Time.get_ticks_msec()
		if _capture:
			_capture.clear_buffer()
		_pending.clear()
		_enc_state = [0, 0]
	talking = want


## The person a whisper would reach (the nearest within reach), or -1.
func _nearest() -> int:
	var best := -1
	var best_d := WHISPER_PX
	for id in game.remotes:
		if game.kinds.get(id) != Protocol.KIND_PLAYER or not game.remotes[id].visible:
			continue
		var d: float = game.remotes[id].position.distance_to(game.me.position)
		if d <= best_d:
			best_d = d
			best = id
	return best


func _read_mic() -> void:
	var n := _capture.get_frames_available()
	if n <= 0:
		return
	var buf := _capture.get_buffer(n)
	# Down to 16 kHz mono (linear interpolation).
	var step := AudioServer.get_mix_rate() / RATE
	var mono := PackedFloat32Array()
	mono.resize(buf.size())
	for i in buf.size():
		mono[i] = (buf[i].x + buf[i].y) * 0.5
	while _resample_pos < mono.size() - 1:
		var i := int(_resample_pos)
		var f := _resample_pos - i
		_pending.append(lerpf(mono[i], mono[i + 1], f))
		_resample_pos += step
	_resample_pos -= mono.size()
	_encode_pending()


func _encode_pending() -> void:
	while _pending.size() >= FRAME:
		var frame := _pending.slice(0, FRAME)
		_pending = _pending.slice(FRAME)
		var data := Adpcm.encode(frame, _enc_state)
		_seq = (_seq + 1) & 0xffff
		send.emit(_seq, talking == 2, data)
		_last_self = Time.get_ticks_msec()


func dev_talk(kind: int, seconds: float) -> void:
	_dev_kind = kind
	_dev_until = Time.get_ticks_msec() + int(seconds * 1000)


## Dev: a warbling tone at real-time pace instead of the microphone.
func _read_tone() -> void:
	var now := Time.get_ticks_msec()
	var want := int((now - _tone_msec) * RATE / 1000.0)
	_tone_msec += int(want * 1000.0 / RATE)
	for i in want:
		_tone_t += 1.0 / RATE
		_pending.append(0.3 * sin(TAU * (330.0 + 60.0 * sin(TAU * 3.0 * _tone_t)) * _tone_t))
	_encode_pending()


## VoiceFrom: play it from the speaker (or in the ears, for a whisper).
## Entity flags: how drunk a speaker is (0..3).
func set_drunk(id: int, tier: int) -> void:
	if tier == 0:
		_drunk.erase(id)
	else:
		_drunk[id] = tier


func on_voice(p: Dictionary) -> void:
	var id: int = p.speaker
	var entry: Dictionary = _players.get(id, {})
	if entry.is_empty() or entry.whisper != p.whisper or not is_instance_valid(entry.node):
		if not entry.is_empty() and is_instance_valid(entry.node):
			entry.node.queue_free()
		entry = _new_player(id, p.whisper)
		if entry.is_empty():
			return
		_players[id] = entry
	var samples := Adpcm.decode(p.data)
	var pb: AudioStreamGeneratorPlayback = entry.playback
	if pb.get_frames_available() < samples.size():
		return  # too far behind: drop rather than lag
	var frames := PackedVector2Array()
	frames.resize(samples.size())
	for i in samples.size():
		frames[i] = Vector2(samples[i], samples[i])
	pb.push_buffer(frames)
	_heard[id] = [Time.get_ticks_msec(), p.whisper]


func _new_player(id: int, whisper: bool) -> Dictionary:
	var gen := AudioStreamGenerator.new()
	gen.mix_rate = RATE
	gen.buffer_length = 0.5
	var node: Node
	if whisper:
		var flat := AudioStreamPlayer.new()
		flat.stream = gen
		flat.bus = "Voice"
		flat.volume_db = -2.0
		add_child(flat)
		flat.play()
		node = flat
	else:
		var v = _view(id)
		if v == null:
			return {}
		var placed := AudioStreamPlayer2D.new()
		placed.stream = gen
		placed.bus = "Voice"
		placed.max_distance = HEAR_PX
		placed.attenuation = 1.0
		v.add_child(placed)
		placed.play()
		node = placed
	return {"node": node, "playback": node.get_stream_playback(), "whisper": whisper}


func _view(id: int):
	if game == null:
		return null
	return game.remotes.get(id)


## Settings: the input device ("" = the system default).
static func set_input_device(name: String) -> void:
	var list := AudioServer.get_input_device_list()
	AudioServer.input_device = name if list.has(name) else "Default"
