## Dev tool: render every floor's art to PNG (needs a renderer - run
## without --headless):
## godot --path client -s tests/render_maps.gd -- /output/dir
extends SceneTree

const Building = preload("res://map/building.gd")
const MapPainter = preload("res://map/map_painter.gd")


func _init() -> void:
	var args := OS.get_cmdline_user_args()
	var out := args[0] if args.size() > 0 else OS.get_user_data_dir()
	var b = Building.new()
	b.load_path("res://maps/building.json")
	for f in b.floors.size():
		var m = b.get_floor(f)
		if m == null:
			continue
		var vp := SubViewport.new()
		vp.size = Vector2i(m.width * m.tile_px * 2, m.height * m.tile_px * 2)
		vp.transparent_bg = true
		vp.msaa_2d = Viewport.MSAA_4X
		vp.render_target_update_mode = SubViewport.UPDATE_ONCE
		var painter := MapPainter.new()
		painter.scale = Vector2(2, 2)
		vp.add_child(painter)
		painter.paint(m)
		get_root().add_child(vp)
		await process_frame
		await process_frame
		await process_frame
		var path := out.path_join("floor%d.png" % f)
		vp.get_texture().get_image().save_png(path)
		print("floor %d -> %s" % [f, path])
	quit()
