## Player settings, kept in user://settings.cfg: full screen, the world's
## ink effect, the default camera zoom, battery saving (30 frames a second),
## sound volumes (0..1).
extends RefCounted

const PATH := "user://settings.cfg"

static var fullscreen := false
static var mood := true
static var zoom := 1.0
static var battery := false
static var vol_sfx := 0.8
static var vol_ambient := 0.6
static var vol_music := 0.5
static var vol_voice := 0.9
static var mic_device := "Default"
static var _loaded := false


static func load_once() -> void:
	if _loaded:
		return
	_loaded = true
	var cfg := ConfigFile.new()
	if cfg.load(PATH) != OK:
		return
	fullscreen = cfg.get_value("video", "fullscreen", false)
	mood = cfg.get_value("video", "mood", true)
	zoom = clampf(float(cfg.get_value("video", "zoom", 1.0)), 0.6, 2.0)
	battery = bool(cfg.get_value("video", "battery", false))
	vol_sfx = clampf(float(cfg.get_value("audio", "sfx", 0.8)), 0.0, 1.0)
	vol_ambient = clampf(float(cfg.get_value("audio", "ambient", 0.6)), 0.0, 1.0)
	vol_music = clampf(float(cfg.get_value("audio", "music", 0.5)), 0.0, 1.0)
	vol_voice = clampf(float(cfg.get_value("audio", "voice", 0.9)), 0.0, 1.0)
	mic_device = str(cfg.get_value("audio", "mic", "Default"))


static func save() -> void:
	var cfg := ConfigFile.new()
	cfg.set_value("video", "fullscreen", fullscreen)
	cfg.set_value("video", "mood", mood)
	cfg.set_value("video", "zoom", zoom)
	cfg.set_value("video", "battery", battery)
	cfg.set_value("audio", "sfx", vol_sfx)
	cfg.set_value("audio", "ambient", vol_ambient)
	cfg.set_value("audio", "music", vol_music)
	cfg.set_value("audio", "voice", vol_voice)
	cfg.set_value("audio", "mic", mic_device)
	cfg.save(PATH)


static func apply_audio() -> void:
	var a = preload("res://audio/audio.gd").inst
	if a:
		a.set_volumes(vol_sfx, vol_ambient, vol_music)
	var vb := AudioServer.get_bus_index("Voice")
	if vb >= 0:
		AudioServer.set_bus_volume_db(vb, linear_to_db(maxf(vol_voice, 0.0001)))
		AudioServer.set_bus_mute(vb, vol_voice <= 0.001)
	var list := AudioServer.get_input_device_list()
	AudioServer.input_device = mic_device if list.has(mic_device) else "Default"


static func apply_window() -> void:
	var want := DisplayServer.WINDOW_MODE_FULLSCREEN if fullscreen else DisplayServer.WINDOW_MODE_WINDOWED
	if DisplayServer.window_get_mode() != want:
		DisplayServer.window_set_mode(want)


## Frames a second: 60 (30 when saving battery); 20 while the window is in
## the background (the game keeps running, it just draws less). The frame
## is what costs CPU here, so this is the big knob.
static func apply_fps(focused := true) -> void:
	Engine.max_fps = (30 if battery else 60) if focused else 20
