## Procedural pixel art for a floor: textured ground, 3/4 walls, furniture.
## Everything is drawn once into a single Image (one sprite per floor), so the
## cost is paid at load time only. Deterministic: same map -> same picture.
extends RefCounted

const TP := 16
const VARIANTS := 4

var map
var img: Image
var rng := RandomNumberGenerator.new()
var _cache := {}  # key -> Image (16x16 tile texture)


func build(p_map) -> Image:
	map = p_map
	img = Image.create(map.width * TP, map.height * TP, false, Image.FORMAT_RGBA8)
	for y in map.height:
		for x in map.width:
			_ground(x, y)
	for y in map.height:
		for x in map.width:
			if _ch(x, y) == "#":
				_wall(x, y)
	_wall_shadows()
	_props()
	return img


# ----------------------------------------------------------------- helpers

func _ch(x: int, y: int) -> String:
	if x < 0 or y < 0 or x >= map.width or y >= map.height:
		return "#"
	return map.tile_chars[y * map.width + x]


func _type(c: String) -> String:
	return map.legend[c]["type"] if map.legend.has(c) else ""


func _is_wall(x: int, y: int) -> bool:
	return _ch(x, y) == "#"


func _room_type(x: int, y: int) -> String:
	return map.room_types.get(map.room_at_tile(x, y), "")


func _hash(x: int, y: int, salt := 0) -> int:
	var h := (x * 73856093) ^ (y * 19349663) ^ (salt * 83492791)
	return absi(h)


func _rect(r: Rect2i, c: Color) -> void:
	img.fill_rect(r.intersection(Rect2i(0, 0, img.get_width(), img.get_height())), c)


func _px(x: int, y: int, c: Color) -> void:
	if x >= 0 and y >= 0 and x < img.get_width() and y < img.get_height():
		if c.a >= 1.0:
			img.set_pixel(x, y, c)
		else:
			img.set_pixel(x, y, img.get_pixel(x, y).blend(c))


## Darken/lighten a rectangle by blending a translucent color over it.
func _tint(r: Rect2i, c: Color) -> void:
	r = r.intersection(Rect2i(0, 0, img.get_width(), img.get_height()))
	for yy in range(r.position.y, r.end.y):
		for xx in range(r.position.x, r.end.x):
			img.set_pixel(xx, yy, img.get_pixel(xx, yy).blend(c))


func _tile_img(key: String, maker: Callable) -> Image:
	if not _cache.has(key):
		var t := Image.create(TP, TP, false, Image.FORMAT_RGBA8)
		maker.call(t)
		_cache[key] = t
	return _cache[key]


func _blit(t: Image, x: int, y: int) -> void:
	img.blit_rect(t, Rect2i(0, 0, TP, TP), Vector2i(x * TP, y * TP))


func _noise(t: Image, base: Color, amount: float, seed_v: int) -> void:
	var r := RandomNumberGenerator.new()
	r.seed = seed_v
	for yy in TP:
		for xx in TP:
			var d := r.randf_range(-amount, amount)
			t.set_pixel(xx, yy, Color(base.r + d, base.g + d, base.b + d))


# ------------------------------------------------------------------ ground

## Floor under a solid object: the most common walkable neighbour.
func _floor_under(x: int, y: int) -> String:
	var counts := {}
	for d in [Vector2i(0, 1), Vector2i(0, -1), Vector2i(1, 0), Vector2i(-1, 0), Vector2i(1, 1), Vector2i(-1, -1)]:
		var c := _ch(x + d.x, y + d.y)
		if map.legend.has(c) and not map.legend[c]["solid"] and c not in ["D", "G", "B", "g", "E", "S"]:
			counts[c] = counts.get(c, 0) + 1
	var best := "."
	var best_n := 0
	for c in counts:
		if counts[c] > best_n:
			best_n = counts[c]
			best = c
	return best


func _ground(x: int, y: int) -> void:
	var c := _ch(x, y)
	var t := _type(c)
	if c == "#":
		return
	if map.legend.has(c) and map.legend[c]["solid"] and c not in ["F", "~", "L"]:
		c = _floor_under(x, y)  # furniture stands on the room's floor
		t = _type(c)
	var v := _hash(x, y) % VARIANTS
	match c:
		".":
			_blit(_tile_img(". %d" % v, func(im): _floor_office(im, v)), x, y)
		",":
			_blit(_tile_img(", %d" % v, func(im): _floor_carpet(im, v)), x, y)
		":", "k":  # stall doors stand on the bathroom tiles (the door is a node)
			_blit(_tile_img(":", func(im): _floor_bath(im)), x, y)
		"_":
			_blit(_tile_img("_ %d" % v, func(im): _floor_lobby(im, v)), x, y)
		"=":
			_blit(_tile_img("= %d" % v, func(im): _floor_asphalt(im, v)), x, y)
		"v":
			_blit(_tile_img("v %d" % v, func(im): _floor_grass(im, v)), x, y)
		"p":
			_blit(_tile_img("p %d" % (y % 2), func(im): _floor_pavers(im, y % 2)), x, y)
		"z":
			_blit(_tile_img("z %d" % v, func(im): _floor_gravel(im, v)), x, y)
		"F":
			_blit(_tile_img("F %d" % v, func(im): _hedge(im, v)), x, y)
		"~":
			_blit(_tile_img("~", func(im): im.fill(Color("#15171f"))), x, y)
		"e":
			_blit(_tile_img("e", func(im): _diamond_plate(im)), x, y)
		"E":  # threshold; the door panels are nodes (open / closed)
			_blit(_tile_img("E", func(im): _elevator_threshold(im)), x, y)
		"s":
			_blit(_tile_img("s", func(im): _steps(im)), x, y)
		"S":
			_blit(_tile_img("S", func(im): _stairs(im)), x, y)
		"D":
			_door(x, y)
		"G":
			_glass_door(x, y)
		"B":
			_gate(x, y)
		"g":
			_garage(x, y)
		"L":
			_locked_door(x, y)
		_:
			_rect(Rect2i(x * TP, y * TP, TP, TP), Color(map.legend[c]["color"]) if map.legend.has(c) else Color.MAGENTA)


func _floor_office(t: Image, v: int) -> void:
	_noise(t, Color("#cfc7b6"), 0.012, 11 + v)
	for i in TP:
		t.set_pixel(i, TP - 1, Color("#bdb4a1"))
		t.set_pixel(TP - 1, i, Color("#bdb4a1"))
	if v == 1:
		t.set_pixel(5, 9, Color("#c3baa8"))


func _floor_carpet(t: Image, v: int) -> void:
	var r := RandomNumberGenerator.new()
	r.seed = 21 + v
	t.fill(Color("#6d84b0"))
	for i in 40:
		var cc := Color("#7b91bd") if r.randf() < 0.5 else Color("#617aa6")
		t.set_pixel(r.randi() % TP, r.randi() % TP, cc)


func _floor_bath(t: Image) -> void:
	for yy in TP:
		for xx in TP:
			var checker := ((xx / 4) + (yy / 4)) % 2 == 0
			var c := Color("#eef1f3") if checker else Color("#dde3e8")
			if xx % 4 == 3 or yy % 4 == 3:
				c = Color("#c8d0d6")
			t.set_pixel(xx, yy, c)


func _floor_lobby(t: Image, v: int) -> void:
	_noise(t, Color("#d9caa5"), 0.01, 31 + v)
	for i in TP:
		t.set_pixel(i, 0, Color("#c7b68e"))
		t.set_pixel(0, i, Color("#c7b68e"))
	for i in 5:  # soft reflection streak
		t.set_pixel(3 + i + v, 11 - i, Color("#e6dab9"))


func _floor_asphalt(t: Image, v: int) -> void:
	var r := RandomNumberGenerator.new()
	r.seed = 41 + v
	t.fill(Color("#5c5e63"))
	for i in 60:
		var cc := Color("#66686d") if r.randf() < 0.5 else Color("#525459")
		t.set_pixel(r.randi() % TP, r.randi() % TP, cc)


func _floor_grass(t: Image, v: int) -> void:
	var r := RandomNumberGenerator.new()
	r.seed = 51 + v
	t.fill(Color("#5b8f45"))
	for i in 34:
		var xx := r.randi() % TP
		var yy := 1 + r.randi() % (TP - 1)
		var cc := Color("#6ea653") if r.randf() < 0.6 else Color("#4c7d39")
		t.set_pixel(xx, yy, cc)
		t.set_pixel(xx, yy - 1, cc.lightened(0.08))
	if v == 3:  # a few flowers
		t.set_pixel(4, 6, Color("#f4e36b"))
		t.set_pixel(11, 12, Color("#f7f7f7"))


func _floor_pavers(t: Image, row: int) -> void:
	for yy in TP:
		for xx in TP:
			var off := 4 if (yy / 8 + row) % 2 == 1 else 0
			var seam := yy % 8 == 7 or (xx + off) % 8 == 7
			t.set_pixel(xx, yy, Color("#96968f") if seam else Color("#adada6"))


func _floor_gravel(t: Image, v: int) -> void:
	var r := RandomNumberGenerator.new()
	r.seed = 61 + v
	t.fill(Color("#8a8272"))
	for i in 70:
		var cc: Color = [Color("#9b937f"), Color("#766e60"), Color("#a59c88")][r.randi() % 3]
		t.set_pixel(r.randi() % TP, r.randi() % TP, cc)


func _hedge(t: Image, v: int) -> void:
	var r := RandomNumberGenerator.new()
	r.seed = 71 + v
	t.fill(Color("#355f2f"))
	for i in 60:
		var cc := Color("#447a3b") if r.randf() < 0.6 else Color("#2b4f27")
		t.set_pixel(r.randi() % TP, r.randi() % TP, cc)


func _diamond_plate(t: Image) -> void:
	t.fill(Color("#9ea9b1"))
	for yy in range(0, TP, 4):
		for xx in range(0, TP, 4):
			var ox := 2 if (yy / 4) % 2 == 1 else 0
			t.set_pixel((xx + ox) % TP, yy + 1, Color("#c3ccd2"))
			t.set_pixel((xx + ox + 1) % TP, yy + 2, Color("#7f8a93"))


func _elevator_door(t: Image) -> void:
	for yy in TP:
		for xx in TP:
			t.set_pixel(xx, yy, Color("#b9c2c9") if (xx + yy) % 5 else Color("#aab4bc"))
	for yy in TP:
		t.set_pixel(7, yy, Color("#6f7a83"))
		t.set_pixel(8, yy, Color("#dfe5ea"))


func _elevator_threshold(t: Image) -> void:
	t.fill(Color("#3a3f47"))
	for xx in TP:
		t.set_pixel(xx, 0, Color("#8a939c"))
		t.set_pixel(xx, TP - 1, Color("#8a939c"))
		t.set_pixel(xx, 7, Color("#2a2e35"))


## Stairwell flights: treads with a nosing, lighter towards the landing.
func _steps(t: Image) -> void:
	for yy in TP:
		var band := yy % 5
		var c := Color("#a88d6e")
		if band == 0:
			c = Color("#cdb697")
		elif band == 4:
			c = Color("#7d6649")
		for xx in TP:
			t.set_pixel(xx, yy, c)


func _stairs(t: Image) -> void:
	for yy in TP:
		var band := (yy % 4)
		var c := Color("#b59c7c")
		if band == 0:
			c = Color("#d2bc9c")
		elif band == 3:
			c = Color("#8e7659")
		for xx in TP:
			t.set_pixel(xx, yy, c)


## Door: floor of the side it belongs to + jambs.
func _door(x: int, y: int) -> void:
	var horizontal_wall := _is_wall(x - 1, y) or _is_wall(x + 1, y) or _ch(x - 1, y) == "D" or _ch(x + 1, y) == "D"
	var fl := _floor_under(x, y)
	var v := _hash(x, y) % VARIANTS
	match fl:
		",": _blit(_tile_img(", %d" % v, func(im): _floor_carpet(im, v)), x, y)
		":": _blit(_tile_img(":", func(im): _floor_bath(im)), x, y)
		"_": _blit(_tile_img("_ %d" % v, func(im): _floor_lobby(im, v)), x, y)
		"=": _blit(_tile_img("= %d" % v, func(im): _floor_asphalt(im, v)), x, y)
		_: _blit(_tile_img(". %d" % v, func(im): _floor_office(im, v)), x, y)
	var px := x * TP
	var py := y * TP
	var wood := Color("#8a5a36")
	if horizontal_wall:
		# Threshold across the passage + jambs where the wall ends.
		_rect(Rect2i(px, py + 6, TP, 4), Color("#a8764c"))
		_rect(Rect2i(px, py + 6, TP, 1), Color("#c18f63"))
		if _is_wall(x - 1, y):
			_rect(Rect2i(px, py, 2, TP), wood)
		if _is_wall(x + 1, y):
			_rect(Rect2i(px + TP - 2, py, 2, TP), wood)
	else:
		_rect(Rect2i(px + 6, py, 4, TP), Color("#a8764c"))
		_rect(Rect2i(px + 6, py, 1, TP), Color("#c18f63"))
		if _is_wall(x, y - 1):
			_rect(Rect2i(px, py, TP, 2), wood)
		if _is_wall(x, y + 1):
			_rect(Rect2i(px, py + TP - 2, TP, 2), wood)


func _glass_door(x: int, y: int) -> void:
	_blit(_tile_img("_ 0", func(im): _floor_lobby(im, 0)), x, y)
	var px := x * TP
	var py := y * TP
	_rect(Rect2i(px, py + 5, TP, 6), Color("#9fd8ea"))
	_rect(Rect2i(px, py + 5, TP, 1), Color("#dff4fb"))
	_rect(Rect2i(px, py + 10, TP, 1), Color("#6fa9bd"))
	_rect(Rect2i(px + 7, py + 5, 2, 6), Color("#8aa0aa"))
	_px(px + 3, py + 7, Color(1, 1, 1, 0.8))
	_px(px + 11, py + 7, Color(1, 1, 1, 0.8))


func _gate(x: int, y: int) -> void:
	_blit(_tile_img("_ 1", func(im): _floor_lobby(im, 1)), x, y)
	var px := x * TP
	var py := y * TP
	# Turnstile: steel posts left/right, yellow arm (card reader light).
	_rect(Rect2i(px, py + 2, 3, 12), Color("#6f7780"))
	_rect(Rect2i(px + TP - 3, py + 2, 3, 12), Color("#6f7780"))
	_rect(Rect2i(px, py + 2, 3, 1), Color("#a9b1ba"))
	_rect(Rect2i(px + TP - 3, py + 2, 3, 1), Color("#a9b1ba"))
	_rect(Rect2i(px + 3, py + 7, 6, 2), Color("#e0b040"))
	_px(px + 1, py + 4, Color("#4cd964"))


func _garage(x: int, y: int) -> void:
	_blit(_tile_img("= 0", func(im): _floor_asphalt(im, 0)), x, y)
	var px := x * TP
	var py := y * TP
	for i in TP:
		var stripe := ((i + 0) / 3) % 2 == 0
		_rect(Rect2i(px + i, py, 1, 3), Color("#e0b040") if stripe else Color("#2b2b2b"))


func _locked_door(x: int, y: int) -> void:
	var px := x * TP
	var py := y * TP
	_rect(Rect2i(px, py, TP, TP), Color("#6e4128"))
	_rect(Rect2i(px + 2, py + 2, 5, 12), Color("#7d4c30"))
	_rect(Rect2i(px + 9, py + 2, 5, 12), Color("#7d4c30"))
	_rect(Rect2i(px + 7, py + 7, 2, 2), Color("#d8c07a"))
	_rect(Rect2i(px + 4, py + 4, 8, 3), Color("#c0392b"))  # "service only" sign


# ------------------------------------------------------------------- walls

## 3/4 view: dark top cap; where the room below is visible, a lit front face.
func _wall(x: int, y: int) -> void:
	var px := x * TP
	var py := y * TP
	var below := _ch(x, y + 1)
	var face: bool = below != "#" and below != "~" and below != "F" and not (map.legend.has(below) and _type(below) == "locked_door")
	var cap := Color("#3b3f50")
	_rect(Rect2i(px, py, TP, TP), cap)
	# Cap texture + edges next to non-wall tiles.
	if not _is_wall(x, y - 1):
		_rect(Rect2i(px, py, TP, 1), Color("#565c73"))
	if not _is_wall(x - 1, y):
		_rect(Rect2i(px, py, 1, TP), Color("#2c2f3c"))
	if not _is_wall(x + 1, y):
		_rect(Rect2i(px + TP - 1, py, 1, TP), Color("#2c2f3c"))
	if _hash(x, y, 3) % 5 == 0:
		_px(px + 5, py + 4, Color("#434859"))
	if face:
		var fy := py + 7
		_rect(Rect2i(px, fy, TP, 9), Color("#9a948a"))       # plaster
		_rect(Rect2i(px, fy, TP, 1), Color("#b5afa4"))       # top edge light
		_rect(Rect2i(px, py + TP - 2, TP, 2), Color("#5f5a52"))  # skirting
		if _hash(x, y, 7) % 7 == 0:                           # a framed picture / notice
			_rect(Rect2i(px + 4, fy + 2, 7, 4), Color("#5a4632"))
			_rect(Rect2i(px + 5, fy + 3, 5, 2), [Color("#7fb2d8"), Color("#e2c46c"), Color("#9bc98a")][_hash(x, y) % 3])
		if not _is_wall(x - 1, y):
			_rect(Rect2i(px, fy, 1, 9), Color("#7c776e"))
		if not _is_wall(x + 1, y):
			_rect(Rect2i(px + TP - 1, fy, 1, 9), Color("#7c776e"))


## Soft shadow cast by walls onto the floor below / to the right.
func _wall_shadows() -> void:
	for y in map.height:
		for x in map.width:
			if _is_wall(x, y) or _ch(x, y) in ["~", "F"]:
				continue
			var px: int = x * TP
			var py: int = y * TP
			if _is_wall(x, y - 1):
				_tint(Rect2i(px, py, TP, 2), Color(0, 0, 0, 0.22))
				_tint(Rect2i(px, py + 2, TP, 2), Color(0, 0, 0, 0.10))
			if _is_wall(x - 1, y):
				_tint(Rect2i(px, py, 2, TP), Color(0, 0, 0, 0.14))


# ------------------------------------------------------------------- props

const PROPS := ["W", "K", "H", "Q", "T", "P", "R", "N", "A", "U", "V", "X", "C", "J", "O", "|", "Y"]


## Connected components of the same furniture char -> one object each.
func _props() -> void:
	var seen := {}
	var index := 0
	for y in map.height:
		for x in map.width:
			var c := _ch(x, y)
			if c not in PROPS or seen.has(Vector2i(x, y)):
				continue
			var tiles: Array[Vector2i] = []
			var stack: Array[Vector2i] = [Vector2i(x, y)]
			seen[Vector2i(x, y)] = true
			while not stack.is_empty():
				var t: Vector2i = stack.pop_back()
				tiles.append(t)
				for d in [Vector2i(1, 0), Vector2i(-1, 0), Vector2i(0, 1), Vector2i(0, -1)]:
					var n: Vector2i = t + d
					if not seen.has(n) and _ch(n.x, n.y) == c:
						seen[n] = true
						stack.append(n)
			var r := Rect2i(tiles[0], Vector2i.ONE)
			for t in tiles:
				r = r.expand(t).expand(t + Vector2i.ONE)
			_prop(c, r, index)
			index += 1


func _shadow(r: Rect2i) -> void:
	_tint(Rect2i(r.position + Vector2i(2, 2), r.size), Color(0, 0, 0, 0.18))


func _prop(c: String, tr: Rect2i, index: int) -> void:
	var r := Rect2i(tr.position * TP, tr.size * TP)
	match c:
		"W": _desks(tr, r, index)
		"K": _counter(r)
		"H": _shelf(r, index)
		"Q": _sofa(r)
		"T": _table(tr, r)
		"P": _plant(r, index)
		"R": _racks(r, index)
		"N": _bench(r)
		"A": _ashtray(r)
		"U": _toilet(tr, r)
		"V": _sink(r)
		"X": _car(r, index)
		"C": _coffee_machine(r)
		"J": _kitchen_counter(r)
		"O": _fruit_bowl(r)
		"|": _partition(r)
		"Y": _sanitizer(r)


func _desks(tr: Rect2i, r: Rect2i, index: int) -> void:
	var top := Rect2i(r.position.x + 1, r.position.y + 2, r.size.x - 2, r.size.y - 5)
	_shadow(top)
	_rect(top, Color("#a67c52"))
	_rect(Rect2i(top.position.x, top.position.y, top.size.x, 1), Color("#c39a6e"))
	_rect(Rect2i(top.position.x, top.end.y, top.size.x, 3), Color("#7a5634"))
	var screens := [Color("#5fb2e8"), Color("#7fd08a"), Color("#e8c45f"), Color("#b28be8")]
	for i in tr.size.x:
		var tx := tr.position.x + i
		var px := tx * TP
		var py := r.position.y
		# Department desks are hot desks: the computer is the laptop an employee
		# puts there (a separate entity), so only a desk mat. Other desks (HR)
		# have a monitor and keyboard.
		if _room_type(tx, tr.position.y) == "department":
			_rect(Rect2i(px + 3, py + 4, 10, 7), Color("#8c6844"))
		else:
			_rect(Rect2i(px + 3, py + 2, 10, 6), Color("#23262e"))
			_rect(Rect2i(px + 4, py + 3, 8, 4), screens[(_hash(tx, tr.position.y, index)) % screens.size()])
			_rect(Rect2i(px + 7, py + 8, 2, 1), Color("#23262e"))
			_rect(Rect2i(px + 4, py + 10, 8, 2), Color("#d9dbe0"))
		if _hash(tx, tr.position.y, 5) % 4 == 0:
			_rect(Rect2i(px + 13, py + 9, 2, 3), Color("#e8e2d8"))
		# Office chair on the floor tile below, if there is one.
		var below := _ch(tx, tr.end.y)
		if map.legend.has(below) and not map.legend[below]["solid"]:
			var cx := px + 4
			var cy := tr.end.y * TP + 2
			_tint(Rect2i(cx + 1, cy + 7, 9, 2), Color(0, 0, 0, 0.2))
			_rect(Rect2i(cx, cy, 8, 3), Color("#30343f"))      # backrest (seen from above)
			_rect(Rect2i(cx + 1, cy + 3, 6, 4), Color("#454b5a"))  # seat
			_rect(Rect2i(cx + 3, cy + 7, 2, 1), Color("#23262e"))


func _counter(r: Rect2i) -> void:
	var top := Rect2i(r.position.x + 1, r.position.y + 1, r.size.x - 2, r.size.y - 4)
	_shadow(top)
	_rect(top, Color("#c9a37a"))
	_rect(Rect2i(top.position.x, top.position.y, top.size.x, 1), Color("#e0bf98"))
	_rect(Rect2i(top.position.x, top.end.y, top.size.x, 3), Color("#94714c"))
	# Something on the counter: a screen and a bell.
	_rect(Rect2i(top.position.x + 3, top.position.y + 2, 7, 5), Color("#23262e"))
	_rect(Rect2i(top.position.x + 4, top.position.y + 3, 5, 3), Color("#5fb2e8"))
	_rect(Rect2i(top.end.x - 6, top.position.y + 4, 3, 2), Color("#d8c07a"))


func _shelf(r: Rect2i, index: int) -> void:
	var body := Rect2i(r.position.x, r.position.y + 1, r.size.x, r.size.y - 2)
	_shadow(body)
	_rect(body, Color("#6d737e"))
	_rect(Rect2i(body.position.x, body.position.y, body.size.x, 1), Color("#9aa1ad"))
	var goods := [Color("#e74c3c"), Color("#f1c40f"), Color("#3498db"), Color("#2ecc71"), Color("#e67e22"), Color("#9b59b6"), Color("#ecf0f1")]
	var n := 0
	for xx in range(body.position.x + 1, body.end.x - 2, 3):
		for row in 2:
			var c: Color = goods[(_hash(xx, row, index) + n) % goods.size()]
			_rect(Rect2i(xx, body.position.y + 2 + row * 6, 2, 4), c)
			_px(xx, body.position.y + 2 + row * 6, c.lightened(0.3))
			n += 1


func _sofa(r: Rect2i) -> void:
	var c := Color("#5b7fbf")
	_shadow(r)
	_rect(Rect2i(r.position.x, r.position.y + 1, r.size.x, r.size.y - 2), c.darkened(0.25))  # back + arms
	_rect(Rect2i(r.position.x + 3, r.position.y + 7, r.size.x - 6, r.size.y - 10), c)         # seat
	for i in range(1, r.size.x / TP):
		_rect(Rect2i(r.position.x + i * TP, r.position.y + 8, 1, r.size.y - 12), c.darkened(0.15))
	_rect(Rect2i(r.position.x, r.position.y + 1, r.size.x, 1), c.lightened(0.2))


func _table(tr: Rect2i, r: Rect2i) -> void:
	# Chairs around it where there is floor.
	for i in tr.size.x:
		var tx := tr.position.x + i
		for side in [-1, 1]:
			var ty := tr.position.y - 1 if side < 0 else tr.end.y
			var t := _ch(tx, ty)
			if map.legend.has(t) and not map.legend[t]["solid"]:
				var cy := ty * TP + (9 if side < 0 else 1)
				_rect(Rect2i(tx * TP + 4, cy, 8, 6), Color("#6b4f35"))
				_rect(Rect2i(tx * TP + 4, cy + (0 if side < 0 else 5), 8, 1), Color("#4e3926"))
	var top := Rect2i(r.position.x + 1, r.position.y + 1, r.size.x - 2, r.size.y - 3)
	_shadow(top)
	_rect(top, Color("#8b6b4a"))
	_rect(Rect2i(top.position.x, top.position.y, top.size.x, 1), Color("#a8865f"))
	_rect(Rect2i(top.position.x, top.end.y, top.size.x, 2), Color("#6a5037"))
	_rect(Rect2i(top.get_center().x - 2, top.get_center().y - 2, 4, 3), Color("#e8e2d8"))  # papers / cups


func _plant(r: Rect2i, index: int) -> void:
	var cx := r.position.x + 8
	var by := r.position.y + 14
	_tint(Rect2i(cx - 4, by - 1, 9, 2), Color(0, 0, 0, 0.2))
	_rect(Rect2i(cx - 3, by - 5, 7, 5), Color("#b5653c"))
	_rect(Rect2i(cx - 3, by - 5, 7, 1), Color("#cf7d50"))
	var greens := [Color("#3f8a3a"), Color("#56a64d"), Color("#2f6e2c")]
	var rr := RandomNumberGenerator.new()
	rr.seed = 91 + index
	for i in 26:
		var a := rr.randf() * TAU
		var d := rr.randf() * 6.0
		var lx := cx + int(cos(a) * d)
		var ly := by - 9 + int(sin(a) * d * 0.8)
		_rect(Rect2i(lx, ly, 2, 2), greens[rr.randi() % 3])


func _racks(r: Rect2i, index: int) -> void:
	_shadow(r)
	_rect(r, Color("#23262d"))
	for tx in range(r.position.x, r.end.x, TP):
		for ty in range(r.position.y, r.end.y, TP):
			_rect(Rect2i(tx + 1, ty + 1, TP - 2, TP - 2), Color("#2d3139"))
			for row in range(3, 14, 3):
				_rect(Rect2i(tx + 3, ty + row, 8, 1), Color("#1b1d22"))
				var led := Color("#4cd964") if _hash(tx, ty + row, index) % 3 else Color("#5fb2e8")
				_px(tx + 12, ty + row, led)


func _bench(r: Rect2i) -> void:
	_shadow(Rect2i(r.position.x, r.position.y + 4, r.size.x, 8))
	for i in 3:
		_rect(Rect2i(r.position.x, r.position.y + 4 + i * 3, r.size.x, 2), Color("#9a7550"))
	_rect(Rect2i(r.position.x + 2, r.position.y + 12, 2, 3), Color("#3d3d3d"))
	_rect(Rect2i(r.end.x - 4, r.position.y + 12, 2, 3), Color("#3d3d3d"))


func _ashtray(r: Rect2i) -> void:
	var cx := r.position.x + 5
	var cy := r.position.y + 3
	_tint(Rect2i(cx + 1, cy + 11, 7, 2), Color(0, 0, 0, 0.25))
	_rect(Rect2i(cx, cy, 6, 11), Color("#7a7f86"))
	_rect(Rect2i(cx, cy, 6, 2), Color("#c9c1a8"))
	_px(cx + 2, cy, Color("#e8e8e8"))


func _toilet(tr: Rect2i, r: Rect2i) -> void:
	var left_wall := _is_wall(tr.position.x - 1, tr.position.y)
	var tank_x := r.position.x + (1 if left_wall else 11)
	_rect(Rect2i(tank_x, r.position.y + 3, 4, 10), Color("#e6e9ec"))
	var bowl_x := r.position.x + (5 if left_wall else 3)
	_rect(Rect2i(bowl_x, r.position.y + 4, 8, 8), Color("#f7f8f9"))
	_rect(Rect2i(bowl_x + 2, r.position.y + 6, 4, 4), Color("#c9dbe6"))


func _sink(r: Rect2i) -> void:
	var left_wall := _is_wall(r.position.x / TP - 1, r.position.y / TP)
	for ty in range(r.position.y, r.end.y, TP):
		var x0 := r.position.x + (1 if left_wall else 4)
		_rect(Rect2i(x0, ty + 3, 11, 10), Color("#eef3f6"))
		_rect(Rect2i(x0 + 2, ty + 5, 7, 6), Color("#bcd3e0"))
		_px(x0 + (1 if left_wall else 9), ty + 8, Color("#9aa4ab"))
		# Mirror on the wall side.
		_rect(Rect2i(r.position.x + (0 if left_wall else 15), ty + 2, 1, 12), Color("#cfe9f5"))


func _car(r: Rect2i, index: int) -> void:
	var colors := [Color("#c0392b"), Color("#2e5fa8"), Color("#ecf0f1"), Color("#2c2f36"), Color("#9aa3ab"), Color("#27ae60"), Color("#d68910")]
	var body: Color = colors[index % colors.size()]
	# Parking bay lines.
	var line := Color(0.92, 0.92, 0.88, 0.75)
	_tint(Rect2i(r.position.x, r.position.y + 1, r.size.x, 1), line)
	_tint(Rect2i(r.position.x, r.end.y - 2, r.size.x, 1), line)
	_tint(Rect2i(r.position.x + 1, r.position.y + 1, 1, r.size.y - 2), line)
	var b := Rect2i(r.position.x + 2, r.position.y + 4, r.size.x - 4, r.size.y - 8)
	_tint(Rect2i(b.position + Vector2i(2, 3), b.size), Color(0, 0, 0, 0.25))
	# Wheels peeking out.
	for wx in [b.position.x + 6, b.end.x - 12]:
		_rect(Rect2i(wx, b.position.y - 2, 6, 3), Color("#1c1c1c"))
		_rect(Rect2i(wx, b.end.y - 1, 6, 3), Color("#1c1c1c"))
	_rect(b, body)
	_rect(Rect2i(b.position.x + 1, b.position.y, b.size.x - 2, 1), body.lightened(0.3))
	_rect(Rect2i(b.position.x + 1, b.end.y - 1, b.size.x - 2, 1), body.darkened(0.3))
	# Roof with windshield (front = right) and rear window.
	var roof := Rect2i(b.position.x + 12, b.position.y + 3, b.size.x - 22, b.size.y - 6)
	_rect(roof, body.darkened(0.12))
	_rect(Rect2i(roof.end.x, roof.position.y, 5, roof.size.y), Color("#3b5068"))
	_rect(Rect2i(roof.end.x, roof.position.y, 5, 1), Color("#8fb0cc"))
	_rect(Rect2i(roof.position.x - 4, roof.position.y, 4, roof.size.y), Color("#3b5068"))
	# Lights.
	_rect(Rect2i(b.end.x - 2, b.position.y + 2, 2, 3), Color("#f7e7a1"))
	_rect(Rect2i(b.end.x - 2, b.end.y - 5, 2, 3), Color("#f7e7a1"))
	_rect(Rect2i(b.position.x, b.position.y + 2, 2, 3), Color("#d64541"))
	_rect(Rect2i(b.position.x, b.end.y - 5, 2, 3), Color("#d64541"))


func _coffee_machine(r: Rect2i) -> void:
	var x := r.position.x + 2
	var y := r.position.y
	_tint(Rect2i(x + 2, y + 13, 12, 2), Color(0, 0, 0, 0.25))
	_rect(Rect2i(x, y + 1, 12, 13), Color("#2b2b30"))           # body
	_rect(Rect2i(x, y + 1, 12, 1), Color("#4a4a52"))
	_rect(Rect2i(x + 1, y + 2, 10, 3), Color("#6b4a2e"))        # bean hopper
	_rect(Rect2i(x + 2, y + 2, 8, 1), Color("#8a6040"))
	_rect(Rect2i(x + 1, y + 6, 10, 6), Color("#b9c2c9"))        # steel front
	_rect(Rect2i(x + 4, y + 7, 4, 1), Color("#2b2b30"))         # spout
	_rect(Rect2i(x + 4, y + 9, 4, 3), Color("#f4f1ea"))         # cup
	_rect(Rect2i(x + 5, y + 9, 2, 1), Color("#6b4a2e"))         # coffee
	_px(x + 10, y + 7, Color("#e74c3c"))                          # power light
	_px(x + 10, y + 9, Color("#4cd964"))


func _kitchen_counter(r: Rect2i) -> void:
	var top := Rect2i(r.position.x, r.position.y + 1, r.size.x, r.size.y - 4)
	_shadow(top)
	_rect(top, Color("#d8d2c4"))
	_rect(Rect2i(top.position.x, top.position.y, top.size.x, 1), Color("#ece8de"))
	_rect(Rect2i(top.position.x, top.end.y, top.size.x, 3), Color("#8f8778"))
	var x := top.position.x
	var y := top.position.y
	# Mugs and a kettle (the fruit bowl is its own tile, "O").
	for i in 3:
		var mug: Color = [Color("#f4f1ea"), Color("#2e86de"), Color("#e67e22")][i]
		_rect(Rect2i(x + 3 + i * 4, y + 4, 3, 3), mug)
		_px(x + 6 + i * 4, y + 5, mug.darkened(0.3))
	if r.size.x >= 32:
		_rect(Rect2i(x + 20, y + 2, 6, 7), Color("#9aa4ab"))
		_rect(Rect2i(x + 21, y + 2, 4, 1), Color("#c9d1d7"))


## Hand sanitizer: a white wall dispenser with a blue label and a drip tray.
func _sanitizer(r: Rect2i) -> void:
	var x := r.position.x + 4
	var y := r.position.y + 2
	_tint(Rect2i(x + 1, y + 1, 8, 11), Color(0, 0, 0, 0.18))
	_rect(Rect2i(x, y, 8, 10), Color("#f4f7fa"))
	_rect(Rect2i(x, y, 8, 1), Color("#ffffff"))
	_rect(Rect2i(x + 2, y + 3, 4, 3), Color("#3b8fd9"))
	_rect(Rect2i(x + 3, y + 10, 2, 1), Color("#9aa4ab"))
	_rect(Rect2i(x + 1, y + 12, 6, 1), Color("#c5ccd3"))


## Toilet stall partitions: light panels with a darker front face.
func _partition(r: Rect2i) -> void:
	var body := Rect2i(r.position.x, r.position.y, r.size.x, r.size.y - 3)
	_shadow(body)
	_rect(body, Color("#c3c9d1"))
	_rect(Rect2i(body.position.x, body.position.y, body.size.x, 2), Color("#e1e5ea"))
	_rect(Rect2i(r.position.x, r.end.y - 3, r.size.x, 3), Color("#8f97a1"))


## Counter with a big bowl of free fruit ("owocowe czwartki", every day).
func _fruit_bowl(r: Rect2i) -> void:
	var top := Rect2i(r.position.x, r.position.y + 1, r.size.x, r.size.y - 4)
	_shadow(top)
	_rect(top, Color("#d8d2c4"))
	_rect(Rect2i(top.position.x, top.position.y, top.size.x, 1), Color("#ece8de"))
	_rect(Rect2i(top.position.x, top.end.y, top.size.x, 3), Color("#8f8778"))
	var x := r.position.x
	var y := r.position.y + 1
	_rect(Rect2i(x + 2, y + 5, 12, 5), Color("#a0703a"))
	_rect(Rect2i(x + 3, y + 9, 10, 1), Color("#7a5228"))
	for f in [[3, 3, "#e74c3c"], [6, 2, "#f1c40f"], [9, 3, "#27ae60"], [5, 5, "#e67e22"], [8, 5, "#c0392b"], [11, 4, "#f39c12"]]:
		_rect(Rect2i(x + f[0], y + f[1], 3, 3), Color(f[2]))
	_px(x + 7, y + 1, Color("#5d4037"))

