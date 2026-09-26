## Renders the tile map once into a texture (placeholder colored tiles)
## plus faint room-name labels.
extends Node2D


func build(map, zoom: float) -> void:
	var tp: int = map.tile_px
	var img := Image.create(map.width * tp, map.height * tp, false, Image.FORMAT_RGBA8)
	for y in map.height:
		for x in map.width:
			var c: String = map.tile_chars[y * map.width + x]
			var col := Color.html(map.legend[c]["color"])
			var r := Rect2i(x * tp, y * tp, tp, tp)
			img.fill_rect(r, col)
			if map.legend[c]["solid"]:
				img.fill_rect(Rect2i(r.position.x, r.end.y - 3, tp, 3), col.darkened(0.3))
			else:
				img.fill_rect(Rect2i(r.position.x, r.position.y, tp, 1), col.darkened(0.06))
				img.fill_rect(Rect2i(r.position.x, r.position.y, 1, tp), col.darkened(0.06))
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
		ls.font_size = 28
		ls.font_color = Color(1, 1, 1, 0.35)
		ls.outline_size = 6
		ls.outline_color = Color(0, 0, 0, 0.25)
		l.label_settings = ls
		l.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR
		l.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		l.scale = Vector2.ONE / zoom
		l.size = Vector2(400, 40)
		l.position = center - Vector2(200, 20) / zoom
		add_child(l)
