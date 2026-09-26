## Renders the floor once into a texture (procedural pixel art, map_art.gd)
## plus faint room-name labels.
extends Node2D

const Ink = preload("res://ui/ink_ui.gd")


const MapArt = preload("res://map/map_art.gd")


## `floor_names`: floor index -> name, for "where do these stairs go" labels.
## Room names and stair signs; the game moves this node to a layer above
## the world's ink effect (and shows it with the floor).
var labels := Node2D.new()


func build(map, zoom: float, floor_names := {}) -> void:
	add_child(labels)
	var t0 := Time.get_ticks_msec()
	var tp: int = map.tile_px
	var img: Image = MapArt.new().build(map)
	print("map art floor %d: %d ms" % [map.floor_index, Time.get_ticks_msec() - t0])
	var sprite := Sprite2D.new()
	sprite.texture = ImageTexture.create_from_image(img)
	sprite.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST  # crisp tiles
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
		if map.room_types.get(rid, "") in ["stall", "elevator"]:
			continue  # tiny rooms (toilet stalls, the elevator car): no label
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
		ls.font = Ink.font()
		ls.font_size = 28
		ls.font_color = Color(Ink.PAPER_HI, 0.6)
		ls.outline_size = 8
		ls.outline_color = Color(Ink.INK, 0.55)
		l.label_settings = ls
		l.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR
		l.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		l.scale = Vector2.ONE / zoom
		l.size = Vector2(400, 40)
		l.position = center - Vector2(200, 20) / zoom
		labels.add_child(l)
	# Stairs: where they lead.
	for link in map.links:
		if link.kind != "stairs" or not floor_names.has(link.to_floor):
			continue
		var a: Rect2i = link.rect
		var sl := Label.new()
		sl.text = "▸ " + floor_names[link.to_floor]
		var ls2 := LabelSettings.new()
		ls2.font = Ink.font()
		ls2.font_size = 22
		ls2.font_color = Ink.PAPER_HI
		ls2.outline_size = 7
		ls2.outline_color = Ink.INK
		sl.label_settings = ls2
		sl.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR
		sl.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		sl.scale = Vector2.ONE / zoom
		sl.size = Vector2(300, 30)
		var c2 := (Vector2(a.position) + Vector2(a.size) / 2.0) * tp
		sl.position = c2 - Vector2(150, 15) / zoom
		labels.add_child(sl)
