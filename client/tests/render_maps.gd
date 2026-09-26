## Dev tool: render every floor's pixel art to PNG (headless).
## godot --headless --path client -s tests/render_maps.gd -- /output/dir
extends SceneTree

const Building = preload("res://map/building.gd")
const MapArt = preload("res://map/map_art.gd")


func _init() -> void:
	var args := OS.get_cmdline_user_args()
	var out := args[0] if args.size() > 0 else OS.get_user_data_dir()
	var b = Building.new()
	b.load_path("res://maps/building.json")
	for f in b.floors.size():
		var m = b.get_floor(f)
		if m == null:
			continue
		var t0 := Time.get_ticks_msec()
		var img: Image = MapArt.new().build(m)
		var path := out.path_join("floor%d.png" % f)
		img.save_png(path)
		print("floor %d: %dx%d in %d ms -> %s" % [f, img.get_width(), img.get_height(), Time.get_ticks_msec() - t0, path])
	quit()
