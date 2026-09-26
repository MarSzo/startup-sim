## Player settings, kept in user://settings.cfg: full screen, the world's
## ink effect, the default camera zoom.
extends RefCounted

const PATH := "user://settings.cfg"

static var fullscreen := false
static var mood := true
static var zoom := 1.0
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


static func save() -> void:
	var cfg := ConfigFile.new()
	cfg.set_value("video", "fullscreen", fullscreen)
	cfg.set_value("video", "mood", mood)
	cfg.set_value("video", "zoom", zoom)
	cfg.save(PATH)


static func apply_window() -> void:
	var want := DisplayServer.WINDOW_MODE_FULLSCREEN if fullscreen else DisplayServer.WINDOW_MODE_WINDOWED
	if DisplayServer.window_get_mode() != want:
		DisplayServer.window_set_mode(want)
