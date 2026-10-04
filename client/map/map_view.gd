## Renders the floor once into a texture (hand-drawn art, map_painter.gd,
## drawn in a SubViewport at ART_SCALE x resolution) plus the stairs' labels
## (room names are on the door plaques).
extends Node2D

const Ink = preload("res://ui/ink_ui.gd")


const MapPainter = preload("res://map/map_painter.gd")
## Resolution of the floor picture: pixels per world pixel.
const ART_SCALE := 3


## `floor_names`: floor index -> name, for "where do these stairs go" labels.
## Stair signs; the game moves this node to a layer above
## the world's ink effect (and shows it with the floor).
var labels := Node2D.new()


func build(map, zoom: float, floor_names := {}) -> void:
	add_child(labels)
	var tp: int = map.tile_px
	# Draw the floor once into a texture (vector art, antialiased).
	var vp := SubViewport.new()
	vp.size = Vector2i(map.width * tp * ART_SCALE, map.height * tp * ART_SCALE)
	vp.transparent_bg = true
	vp.disable_3d = true
	vp.msaa_2d = Viewport.MSAA_4X
	vp.render_target_update_mode = SubViewport.UPDATE_ONCE
	var painter := MapPainter.new()
	painter.scale = Vector2(ART_SCALE, ART_SCALE)
	vp.add_child(painter)
	painter.paint(map)
	add_child(vp)
	var sprite := Sprite2D.new()
	sprite.texture = vp.get_texture()
	sprite.scale = Vector2.ONE / ART_SCALE
	sprite.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR
	sprite.centered = false
	add_child(sprite)
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
		sl.set_meta("anchor", c2)
		sl.set_meta("half", Vector2(150, 15))
		labels.add_child(sl)


## Camera zoom changed: keep the labels the same size on screen.
func set_zoom(zoom: float) -> void:
	for l in labels.get_children():
		l.scale = Vector2.ONE / zoom
		l.position = l.get_meta("anchor") - l.get_meta("half") / zoom
