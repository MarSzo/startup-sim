## Renders the floor once into a texture (procedural pixel art, map_art.gd)
## plus faint room-name labels.
extends Node2D


const MapArt = preload("res://map/map_art.gd")


func build(map, zoom: float) -> void:
	var t0 := Time.get_ticks_msec()
	var tp: int = map.tile_px
	var img: Image = MapArt.new().build(map)
	print("map art floor %d: %d ms" % [map.floor_index, Time.get_ticks_msec() - t0])
	var sprite := Sprite2D.new()
	sprite.texture = ImageTexture.create_from_image(img)
	sprite.centered = false
	add_child(sprite)
	# Room labels at each room's centroid.
	var sums := {}
	for i in map.room.size():
		var rid: int = map.room[i]
		if rid == 0:
			continue
		if not sums.has(rid):
			sums[rid] = [Vector2.ZERO, 0]
		sums[rid][0] += Vector2(i % map.width, i / map.width)
		sums[rid][1] += 1
	for rid in sums:
		if map.room_types.get(rid, "") == "stall":
			continue  # tiny rooms inside a bathroom: no label
		var c: Vector2 = sums[rid][0] / float(sums[rid][1])
		# Non-convex areas (the outside wraps the building): use the room's
		# tile nearest to the centroid.
		if map.room_at_tile(int(c.x), int(c.y)) != rid:
			var best := Vector2i.ZERO
			var best_d := INF
			for i in map.room.size():
				if map.room[i] == rid:
					var t := Vector2(i % map.width, i / map.width)
					if t.distance_squared_to(c) < best_d:
						best_d = t.distance_squared_to(c)
						best = Vector2i(t)
			c = Vector2(best)
		var center: Vector2 = (c + Vector2(0.5, 0.5)) * tp
		var l := Label.new()
		l.text = map.room_name(rid)
		var ls := LabelSettings.new()
		ls.font_size = 22
		ls.font_color = Color(1, 1, 1, 0.28)
		ls.outline_size = 6
		ls.outline_color = Color(0, 0, 0, 0.25)
		l.label_settings = ls
		l.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR
		l.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		l.scale = Vector2.ONE / zoom
		l.size = Vector2(400, 40)
		l.position = center - Vector2(200, 20) / zoom
		add_child(l)
