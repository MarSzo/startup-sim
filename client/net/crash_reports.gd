## Client crash reports. Every session marks itself "running" in
## user://session.cfg and "clean" on a normal exit; finding "running" at the
## next start means the game crashed (or was killed). The previous session's
## log (Godot keeps it as logs/godot<date>.log) then goes, with the player's
## consent, to the server (POST /api/crash, see server/src/crash.rs).
## Only exported builds (in the editor every stopped run would count), or
## with --crash-test.
extends RefCounted

const STATE := "user://session.cfg"
const LOGS := "user://logs"
## The end of the log that goes in a report.
const LOG_TAIL := 48 * 1024


static func enabled(args: Dictionary) -> bool:
	return not OS.has_feature("editor") or args.has("crash-test")


## At start-up: did the previous session crash? Returns {started} if so,
## else {}; marks this session as running.
static func begin_session() -> Dictionary:
	var cfg := ConfigFile.new()
	var crashed := {}
	if cfg.load(STATE) == OK and cfg.get_value("session", "running", false):
		crashed = {"started": str(cfg.get_value("session", "started", ""))}
	cfg.set_value("session", "running", true)
	cfg.set_value("session", "started", Time.get_datetime_string_from_system(true) + " UTC")
	cfg.save(STATE)
	return crashed


## A normal exit.
static func end_session() -> void:
	var cfg := ConfigFile.new()
	cfg.load(STATE)
	cfg.set_value("session", "running", false)
	cfg.save(STATE)


## The end of the previous session's log ("" if there is none).
static func previous_log() -> String:
	var dir := DirAccess.open(LOGS)
	if dir == null:
		return ""
	var newest := ""
	for f in dir.get_files():
		# godot.log is this session's; older ones carry their date in the name.
		if f.begins_with("godot") and f.ends_with(".log") and f != "godot.log" and f > newest:
			newest = f
	if newest == "":
		return ""
	var bytes := FileAccess.get_file_as_bytes(LOGS.path_join(newest))
	if bytes.size() > LOG_TAIL:
		bytes = bytes.slice(bytes.size() - LOG_TAIL)
	return bytes.get_string_from_utf8()


## The report: version, system, hardware, the log.
static func make_report(crashed: Dictionary, log_text: String) -> Dictionary:
	return {
		"version": "%s (Godot %s)" % [ProjectSettings.get_setting("application/config/version", "?"), Engine.get_version_info().string],
		"os": "%s %s" % [OS.get_name(), OS.get_version()],
		"cpu": OS.get_processor_name(),
		"gpu": RenderingServer.get_video_adapter_name(),
		"started": crashed.get("started", ""),
		"log": log_text,
	}
