## Hand-drawn character (+ nick label and speech bubble). Position is in world
## px; the node origin is the feet (collision box center).
## Appearance comes from a seed (the entity id) or a fixed look for NPC staff;
## the walk cycle is driven by the distance actually travelled, so it matches
## the movement for both the local player and interpolated remote ones.
extends Node2D

const Ink = preload("res://ui/ink_ui.gd")
const ItemArt = preload("res://game/item_art.gd")

const FACING_DOWN := 0
const FACING_UP := 1
const FACING_LEFT := 2
const FACING_RIGHT := 3
const BUBBLE_WIDTH := 260.0
const HEAD_TOP := -27.0  # top of the head relative to the feet

## Appearance (entity flags bits 3..5): 0 player, 1 porter (uniform + cap),
## 2 office staff (shirt + tie).
const LOOK_PLAYER := 0
const LOOK_PORTER := 1
const LOOK_OFFICE := 2
const LOOK_GUARD := 3      # shop security
const LOOK_POLICE := 4
const LOOK_CLEANER := 5
const LOOK_FIREFIGHTER := 6

const SKINS := [Color("#f2cfae"), Color("#e3b08c"), Color("#c68c63"), Color("#8d5a3b")]
const HAIRS := [Color("#2b2118"), Color("#5a3b22"), Color("#a0703a"), Color("#d9b66b"), Color("#8a8a8a"), Color("#b5462e"), Color("#1d1d27")]
const SHIRTS := [Color("#d64541"), Color("#2e86de"), Color("#27ae60"), Color("#f39c12"), Color("#8e44ad"), Color("#16a085"), Color("#e84393"), Color("#f5f6fa"), Color("#34495e"), Color("#c0a16b")]
const PANTS := [Color("#2f3a56"), Color("#3b3b3b"), Color("#5a4a3a"), Color("#4a5a3a"), Color("#6b7a8f")]

var look := LOOK_PLAYER
var facing := FACING_DOWN
## Activity (Protocol.STATUS_*): brewing at the machine.
const ACT_COMPUTER := 1
const ACT_BREWING := 2
const ACT_SOFA := 3
const ACT_TOILET := 4
const ACT_SMOKING := 5
const ACT_WASHING := 6

var slow := false
var smelly := false
var umbrella := false
var status := 0
## Item in hands (ItemArt kinds), visible to everyone.
var held := 0
## White outline marks the local player.
var highlight := false
var skin := SKINS[0]
var hair := HAIRS[0]
var hair_style := 0   # 0 short, 1 long, 2 bun, 3 spiky, 4 ponytail, 5 bald
var shirt := SHIRTS[0]
var pants := PANTS[0]
var tie := Color("#c0392b")

var nick_label := Label.new()
## Nick and speech bubble live in `_tag`, which follows this view from a
## layer above the world's ink effect (`label_root`) so text stays sharp.
static var label_root: Node = null
var _tag := Node2D.new()
var bubble := PanelContainer.new()
var bubble_label := Label.new()
var _bubble_time := 0.0
var _zoom := 1.0
var _last_pos := Vector2.INF
var _walk := 0.0      # distance-driven walk phase
var _idle := 1.0      # seconds since the last movement


func setup(seed_id: int, nick: String, zoom: float) -> void:
	_zoom = zoom
	set_seed(seed_id)
	nick_label.text = nick
	var ls := LabelSettings.new()
	ls.font = Ink.font()
	ls.font_size = 20
	ls.font_color = Ink.PAPER_HI
	ls.outline_size = 6
	ls.outline_color = Ink.INK
	nick_label.label_settings = ls
	nick_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	nick_label.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR
	# Render the label at screen resolution regardless of camera zoom.
	nick_label.scale = Vector2.ONE / zoom
	nick_label.size = Vector2(200, 24)
	nick_label.position = Vector2(-100 / zoom, HEAD_TOP - 24 / zoom)
	if nick_label.get_parent() == null:
		add_child(_tag)
		_tag.add_child(nick_label)
		_build_bubble()
	queue_redraw()


const HAIR_STYLE_NAMES := ["krótkie", "długie", "kok", "jeżyk", "kucyk", "łysa głowa"]


## Chosen look (character creator / PlayerInfo): palette indices.
func set_appearance(a: Dictionary) -> void:
	skin = SKINS[clampi(a.skin, 0, SKINS.size() - 1)]
	hair_style = clampi(a.hair_style, 0, HAIR_STYLE_NAMES.size() - 1)
	hair = HAIRS[clampi(a.hair_color, 0, HAIRS.size() - 1)]
	shirt = SHIRTS[clampi(a.shirt, 0, SHIRTS.size() - 1)]
	pants = PANTS[clampi(a.pants, 0, PANTS.size() - 1)]
	queue_redraw()


## Deterministic look from an id (players) - same on every client.
func set_seed(seed_id: int) -> void:
	var h := absi(seed_id * 2654435761) >> 3
	skin = SKINS[h % SKINS.size()]
	hair = HAIRS[(h / 7) % HAIRS.size()]
	hair_style = (h / 53) % 6
	shirt = SHIRTS[(h / 211) % SHIRTS.size()]
	pants = PANTS[(h / 1237) % PANTS.size()]
	tie = [Color("#c0392b"), Color("#2e86de"), Color("#27ae60")][(h / 17) % 3]
	match look:
		LOOK_PORTER:
			shirt = Color("#2c3e6b")
			pants = Color("#1f2a44")
		LOOK_OFFICE:
			shirt = Color("#f4f6f8")
			pants = Color("#2d3036")
		LOOK_GUARD:
			shirt = Color("#23262b")
			pants = Color("#1a1c20")
		LOOK_POLICE:
			shirt = Color("#1f3358")
			pants = Color("#17233d")
		LOOK_CLEANER:
			shirt = Color("#2bb3a8")
			pants = Color("#3d4f5c")
		LOOK_FIREFIGHTER:
			shirt = Color("#1d2433")
			pants = Color("#1d2433")
	queue_redraw()


func _build_bubble() -> void:
	bubble.add_theme_stylebox_override("panel", Ink.box("bubble"))
	bubble_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	bubble_label.custom_minimum_size = Vector2(BUBBLE_WIDTH, 0)
	Ink.style_label(bubble_label, 16, Ink.TEXT_INK)
	bubble.add_child(bubble_label)
	bubble.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR
	bubble.scale = Vector2.ONE / _zoom
	bubble.visible = false
	bubble.z_index = 10
	_tag.add_child(bubble)


func set_nick(nick: String) -> void:
	nick_label.text = nick


## Show a speech bubble above the head for a few seconds.
func say(text: String) -> void:
	bubble_label.text = text
	bubble.reset_size()
	bubble.visible = true
	_bubble_time = 4.0 + text.length() * 0.07
	_place_bubble()


func _place_bubble() -> void:
	var sz := bubble.get_combined_minimum_size() / _zoom
	bubble.position = Vector2(-sz.x / 2, HEAD_TOP - 28 / _zoom - sz.y)


func set_held(k: int) -> void:
	if k != held:
		held = k
		queue_redraw()


## Open umbrella over the head (outdoors in the rain).
func set_umbrella(on: bool) -> void:
	if on != umbrella:
		umbrella = on
		queue_redraw()


## Low hygiene: a green smell cloud (everyone sees it).
func set_smelly(on: bool) -> void:
	if on != smelly:
		smelly = on
		queue_redraw()


## Activity (Protocol.ACT_*) and the slow walk (tired / needs the toilet).
func set_status(s: int, p_slow := false) -> void:
	if s != status or p_slow != slow:
		status = s
		slow = p_slow
		queue_redraw()


func set_facing(f: int) -> void:
	if f != facing:
		facing = f
		queue_redraw()


## Camera zoom changed: nick and bubble stay the same size on screen.
func set_zoom(zoom: float) -> void:
	_zoom = zoom
	nick_label.scale = Vector2.ONE / zoom
	nick_label.position = Vector2(-100 / zoom, HEAD_TOP - 24 / zoom)
	bubble.scale = Vector2.ONE / zoom
	if bubble.visible:
		_place_bubble()


func _process(delta: float) -> void:
	# In the game world (not e.g. a video call tile): move the text up.
	if _tag.get_parent() == self and label_root and is_instance_valid(label_root) \
			and label_root.get_parent() and label_root.get_parent().is_ancestor_of(self):
		_tag.reparent(label_root, false)
		tree_exiting.connect(func(): if is_instance_valid(_tag): _tag.queue_free())
	if _tag.get_parent() != self:
		_tag.global_position = global_position
		_tag.visible = is_visible_in_tree()
	if bubble.visible:
		_bubble_time -= delta
		if _bubble_time <= 0.0:
			bubble.visible = false
		else:
			_place_bubble()
	if status in [ACT_BREWING, ACT_SOFA, ACT_SMOKING, ACT_COMPUTER, ACT_WASHING] or slow or smelly:
		queue_redraw()  # animated dots / zzz / smoke / sweat
	# Walk cycle from the distance travelled since the last frame.
	if _last_pos != Vector2.INF:
		var d := position.distance_to(_last_pos)
		if d > 0.01 and d < 8.0:
			_walk += d / 3.5
			_idle = 0.0
			queue_redraw()
		else:
			if _idle < 0.12 and _idle + delta >= 0.12:
				queue_redraw()  # settle into the standing pose
			_idle += delta
	_last_pos = position


func _frame() -> int:
	return 0 if _idle >= 0.12 else int(_walk) % 4


# ------------------------------------------------------------------- drawing
# Hand-drawn look (Don't Starve-ish): a big round head with big eyes, a small
# trapezoid body, stick-thin limbs, everything with an ink outline. Units are
# world pixels; the origin is between the feet.

const OL := 0.75                 # ink outline width
const INK := Color("#1d1712")
const HEAD_R := 6.2
const HEAD_Y := -20.5            # head centre (standing)


func _ink() -> Color:
	return Color("#fff6dc") if highlight else INK


## A limb: a stick with round ends, outlined.
func _limb(a: Vector2, b: Vector2, w: float, col: Color) -> void:
	var ink := _ink()
	draw_line(a, b, ink, w + OL * 2, true)
	draw_circle(a, w / 2 + OL, ink)
	draw_circle(b, w / 2 + OL, ink)
	draw_line(a, b, col, w, true)
	draw_circle(a, w / 2, col)
	draw_circle(b, w / 2, col)


func _blob(c: Vector2, r: float, col: Color) -> void:
	draw_circle(c, r + OL, _ink())
	draw_circle(c, r, col)


## A filled polygon with an ink outline.
func _shape(pts: PackedVector2Array, col: Color, outline := true) -> void:
	draw_colored_polygon(pts, col)
	if outline:
		var closed := pts.duplicate()
		closed.append(pts[0])
		draw_polyline(closed, _ink(), OL * 1.4, true)


## Points of an arc of the head circle (angles in radians, 0 = right, y down).
func _arc_pts(c: Vector2, r: float, a0: float, a1: float, n := 14) -> PackedVector2Array:
	var pts := PackedVector2Array()
	for k in n + 1:
		var a := lerpf(a0, a1, float(k) / n)
		pts.append(c + Vector2(cos(a), sin(a)) * r)
	return pts


func _draw() -> void:
	var walking := _idle < 0.12
	var phase := _walk * PI / 2.0
	var swing := sin(phase) if walking else 0.0
	var bob := absf(sin(phase)) * 0.8 if walking else 0.0
	var side := facing == FACING_LEFT or facing == FACING_RIGHT
	var dir := -1.0 if facing == FACING_LEFT else 1.0
	var back := facing == FACING_UP
	var sit := status in [ACT_SOFA, ACT_TOILET, ACT_COMPUTER] and not walking
	var drop := 3.0 if sit else 0.0
	var ink := _ink()

	# Soft shadow at the feet.
	draw_set_transform(Vector2(0, 0.6), 0.0, Vector2(1.0, 0.35))
	draw_circle(Vector2.ZERO, 5.5, Color(0, 0, 0, 0.3))
	draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)

	var hip := Vector2(0, -7.0 + drop - bob)
	var shoulder_y := -13.5 + drop - bob
	var head := Vector2(0, HEAD_Y + drop - bob)

	# Legs: thin sticks, stepping; seated = short and bent forward.
	var shoe := Color("#231a14")
	if sit:
		for sx in [-1.4, 1.4]:
			var knee := hip + Vector2(sx, 1.5)
			_limb(hip + Vector2(sx, 0), knee + Vector2(dir if side else 0.0, 1.0), 1.3, pants)
	elif side:
		var fwd := swing * 2.2
		_limb(hip + Vector2(-0.3, 0), Vector2(-fwd * dir * 0.6, -0.8), 1.4, pants.darkened(0.15))
		_limb(hip + Vector2(0.3, 0), Vector2(fwd * dir * 0.6, -0.8), 1.4, pants)
		_blob(Vector2(-fwd * dir * 0.6 + dir * 0.6, -0.4), 0.9, shoe)
		_blob(Vector2(fwd * dir * 0.6 + dir * 0.6, -0.4), 0.9, shoe)
	else:
		var lift_l := maxf(swing, 0.0) * 1.2
		var lift_r := maxf(-swing, 0.0) * 1.2
		_limb(hip + Vector2(-1.4, 0), Vector2(-1.7, -0.8 - lift_l), 1.4, pants)
		_limb(hip + Vector2(1.4, 0), Vector2(1.7, -0.8 - lift_r), 1.4, pants.darkened(0.12))
		_blob(Vector2(-1.8, -0.5 - lift_l), 0.95, shoe)
		_blob(Vector2(1.8, -0.5 - lift_r), 0.95, shoe)

	# Arms behind the body when seen from the side (far arm).
	var arm_swing := -swing * 1.6
	var hand_col := skin
	if side:
		_limb(Vector2(-dir * 0.8, shoulder_y + 1), Vector2(-dir * 0.8 - arm_swing * dir * 0.5, hip.y + 0.5), 1.1, shirt.darkened(0.25))

	# Body: a small trapezoid, a bit wider at the shoulders.
	var sw := 3.6 if not side else 2.6
	var hw := 3.0 if not side else 2.4
	var body := PackedVector2Array([
		Vector2(-sw, shoulder_y), Vector2(sw, shoulder_y),
		Vector2(hw, hip.y + 0.6), Vector2(-hw, hip.y + 0.6)])
	_shape(body, shirt)
	# Clothing details.
	if not back:
		match look:
			LOOK_OFFICE:
				var tx := dir * 1.0 if side else 0.0
				_shape(PackedVector2Array([Vector2(tx - 0.6, shoulder_y + 0.3), Vector2(tx + 0.6, shoulder_y + 0.3),
					Vector2(tx + 0.9, hip.y - 1.0), Vector2(tx, hip.y), Vector2(tx - 0.9, hip.y - 1.0)]), tie, false)
			LOOK_CLEANER:
				_shape(PackedVector2Array([Vector2(-2.3, shoulder_y + 1.5), Vector2(2.3, shoulder_y + 1.5),
					Vector2(2.6, hip.y + 0.5), Vector2(-2.6, hip.y + 0.5)]), Color("#eef5f0"))
			LOOK_POLICE, LOOK_PORTER:
				if not side:
					draw_circle(Vector2(1.5, shoulder_y + 1.8), 0.7, Color("#e0b84a"))
	match look:
		LOOK_FIREFIGHTER:
			draw_line(Vector2(-sw + 0.3, shoulder_y + 2.2), Vector2(sw - 0.3, shoulder_y + 2.2), Color("#f1e05a"), 0.9)
			draw_line(Vector2(-hw, hip.y - 1.2), Vector2(hw, hip.y - 1.2), Color("#f1e05a"), 0.9)
		LOOK_GUARD:
			draw_line(Vector2(-sw + 0.2, shoulder_y + 2.0), Vector2(sw - 0.2, shoulder_y + 2.0), Color("#f1c40f"), 1.4)
		LOOK_POLICE, LOOK_PORTER:
			draw_line(Vector2(-hw, hip.y - 0.4), Vector2(hw, hip.y - 0.4), Color("#141414") if look == LOOK_POLICE else Color("#d4ac2b"), 0.8)

	# Arms: thin, swinging opposite to the legs; hands as small blobs.
	if side:
		var hand := Vector2(dir * 0.8 + arm_swing * dir * 0.5, hip.y + 0.5)
		_limb(Vector2(dir * 0.8, shoulder_y + 1), hand, 1.1, shirt.darkened(0.12))
		_blob(hand, 0.8, hand_col)
	else:
		var hl := Vector2(-sw - 0.6, hip.y + 0.3 + maxf(arm_swing, 0.0) * 0.4)
		var hr := Vector2(sw + 0.6, hip.y + 0.3 + maxf(-arm_swing, 0.0) * 0.4)
		_limb(Vector2(-sw + 0.3, shoulder_y + 0.8), hl, 1.1, shirt.darkened(0.12))
		_limb(Vector2(sw - 0.3, shoulder_y + 0.8), hr, 1.1, shirt.darkened(0.12))
		_blob(hl, 0.8, hand_col)
		_blob(hr, 0.8, hand_col)
	if look == LOOK_CLEANER:
		var mx := 6.0 if not side else dir * 5.0
		_limb(Vector2(mx, shoulder_y - 1), Vector2(mx, 0), 0.6, Color("#a0764b"))
		_shape(PackedVector2Array([Vector2(mx - 2.4, 0), Vector2(mx + 2.4, 0), Vector2(mx + 1.6, -1.6), Vector2(mx - 1.6, -1.6)]), Color("#d9d4c7"))

	# Head: big and round.
	_blob(head, HEAD_R, skin)
	# Ears (front view).
	if not side and not back:
		_blob(head + Vector2(-HEAD_R + 0.2, 0.8), 1.0, skin)
		_blob(head + Vector2(HEAD_R - 0.2, 0.8), 1.0, skin)
		draw_circle(head, HEAD_R, skin)  # hide the inner ear outline
	_draw_hair(head, side, dir)
	# Face: big dark eyes with a glint, a small mouth.
	if not back:
		var eyes: Array = [Vector2(-2.2, 0.6), Vector2(2.2, 0.6)] if not side else [Vector2(dir * 2.6, 0.6)]
		for e in eyes:
			var ep: Vector2 = head + e
			draw_set_transform(ep, 0.0, Vector2(0.8, 1.0))
			draw_circle(Vector2.ZERO, 1.25, INK)
			draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)
			draw_circle(ep + Vector2(-0.35, -0.45), 0.38, Color(1, 1, 1, 0.9))
		var tired := slow or status == ACT_SOFA
		if side:
			draw_line(head + Vector2(dir * 5.6, 1.6), head + Vector2(dir * 6.4, 2.3), INK, 0.5, true)  # nose
			draw_line(head + Vector2(dir * 2.6, 3.6), head + Vector2(dir * 3.8, 3.4), INK, 0.45, true)
		elif tired:
			draw_line(head + Vector2(-1.0, 3.6), head + Vector2(1.0, 3.6), INK, 0.45, true)
		else:
			draw_arc(head + Vector2(0, 2.6), 1.2, 0.3, PI - 0.3, 8, INK, 0.45, true)
	_draw_headwear(head, side, dir)
	_draw_status(head, shoulder_y, hip.y, side, dir)


## Hair over the head: short, long, bun, spiky (Wilson-like), ponytail, bald.
func _draw_hair(head: Vector2, side: bool, dir: float) -> void:
	if look in [LOOK_PORTER, LOOK_POLICE, LOOK_FIREFIGHTER]:
		return  # under the cap / helmet
	var h := hair
	var r := HEAD_R + 0.5
	if hair_style == 5:  # bald: a shine
		draw_arc(head + Vector2(-1.5, -2.5), 1.6, PI * 1.1, PI * 1.6, 6, Color(1, 1, 1, 0.5), 0.6, true)
		return
	var back := facing == FACING_UP
	if back:
		_shape(_arc_pts(head, r, PI * 0.05, PI * 0.95 + PI, 20), h)
		if hair_style == 1:
			_shape(PackedVector2Array([head + Vector2(-5.5, 0), head + Vector2(5.5, 0), head + Vector2(4.8, 8), head + Vector2(-4.8, 8)]), h)
		elif hair_style == 4:
			_limb(head + Vector2(0, 3), head + Vector2(0, 9), 1.8, h)
		elif hair_style == 2:
			_blob(head + Vector2(0, -r - 0.6), 2.0, h)
		return
	# Front / side: a cap of hair over the top of the head.
	var a0 := PI * 1.05
	var a1 := PI * 1.95
	if side:
		a0 = PI * (1.0 if dir > 0 else 1.25) - (0.35 if dir > 0 else 0.0)
		a1 = PI * (1.75 if dir > 0 else 2.0) + (0.0 if dir > 0 else 0.35)
	var cap := _arc_pts(head, r, a0, a1, 16)
	cap.append(head + Vector2(cos(a1), sin(a1)) * (r - 3.2) + Vector2(0, 0.6))
	cap.append(head + Vector2(0, -1.2))
	cap.append(head + Vector2(cos(a0), sin(a0)) * (r - 3.2) + Vector2(0, 0.6))
	match hair_style:
		1:  # long: down to the shoulders
			var l := PackedVector2Array([head + Vector2(-r, -1), head + Vector2(-r + 0.4, 6.5), head + Vector2(-r + 2.4, 6.0), head + Vector2(-r + 2.0, 0)])
			var rr := PackedVector2Array([head + Vector2(r, -1), head + Vector2(r - 0.4, 6.5), head + Vector2(r - 2.4, 6.0), head + Vector2(r - 2.0, 0)])
			if not side or dir < 0:
				_shape(rr, h)
			if not side or dir > 0:
				_shape(l, h)
		2:  # bun
			_blob(head + Vector2(0, -r - 0.8), 2.1, h)
		4:  # ponytail (behind, to the side)
			var px := -dir * (r - 0.5) if side else r - 0.5
			_limb(head + Vector2(px, -1), head + Vector2(px + (-dir if side else 1.0) * 1.5, 5), 1.8, h)
	_shape(cap, h)
	if hair_style == 3:  # spikes
		for k in 5:
			var a := lerpf(PI * 1.1, PI * 1.9, k / 4.0)
			var base := head + Vector2(cos(a), sin(a)) * (r - 0.6)
			var tip := head + Vector2(cos(a), sin(a)) * (r + 3.0)
			_shape(PackedVector2Array([base + Vector2(-1.1, 0.3).rotated(a + PI / 2), tip, base + Vector2(1.1, -0.3).rotated(a + PI / 2)]), h)
	# Shine.
	draw_arc(head, r - 1.4, PI * 1.25, PI * 1.45, 5, Color(h.lightened(0.35), 0.8), 0.6, true)


## Caps and helmets.
func _draw_headwear(head: Vector2, side: bool, dir: float) -> void:
	var r := HEAD_R + 0.6
	match look:
		LOOK_FIREFIGHTER:
			_shape(_arc_pts(head + Vector2(0, -0.5), r + 0.3, PI, TAU, 16), Color("#d62f2f"))
			_shape(PackedVector2Array([head + Vector2(-r - 1.8, -0.2), head + Vector2(r + 1.8, -0.2), head + Vector2(r + 1.2, 0.9), head + Vector2(-r - 1.2, 0.9)]), Color("#a31f1f"))
			_blob(head + Vector2(0, -r + 1.4), 0.9, Color("#f1e05a"))
		LOOK_POLICE, LOOK_PORTER:
			var cap_col := Color("#17233d") if look == LOOK_POLICE else Color("#1b2440")
			_shape(PackedVector2Array([head + Vector2(-r, -1.0), head + Vector2(-r + 0.6, -r - 0.6), head + Vector2(r - 0.6, -r - 0.6), head + Vector2(r, -1.0)]), cap_col)
			if look == LOOK_POLICE:
				draw_line(head + Vector2(-r + 0.3, -2.0), head + Vector2(r - 0.3, -2.0), Color("#e8e8e8"), 1.0)
			else:
				draw_circle(head + Vector2(0, -3.8), 0.8, Color("#d4ac2b"))
			if facing != FACING_UP:
				var vx := dir * 2.5 if side else 0.0
				_shape(PackedVector2Array([head + Vector2(vx - 3.8, -1.0), head + Vector2(vx + 3.8, -1.0), head + Vector2(vx + 3.0, 0.3), head + Vector2(vx - 3.0, 0.3)]), Color("#0b0f1a"))


## Things in hands, activities and states: all hand-drawn with ink.
func _draw_status(head: Vector2, shoulder_y: float, hip_y: float, side: bool, dir: float) -> void:
	var ms := Time.get_ticks_msec()
	var top := head.y - HEAD_R
	var hand := Vector2(dir * 2.0 if side else 4.2, hip_y - 0.5)
	if held != 0 and facing != FACING_UP:
		match held:
			ItemArt.COFFEE:
				_shape(PackedVector2Array([hand + Vector2(-1.3, -2.6), hand + Vector2(1.3, -2.6), hand + Vector2(1.1, 0.2), hand + Vector2(-1.1, 0.2)]), Color("#f4f1ea"))
				draw_line(hand + Vector2(-1.1, -2.2), hand + Vector2(1.1, -2.2), Color("#6b4a2e"), 0.7)
				var k := float(ms % 1400) / 1400.0
				draw_arc(hand + Vector2(sin(k * TAU) * 0.6, -4.0 - k * 3.0), 0.8, 0, TAU, 8, Color(1, 1, 1, 0.7 * (1.0 - k)), 0.5, true)
				queue_redraw()
			ItemArt.LAPTOP:
				var lx := dir * 1.5 if side else 0.0
				_shape(PackedVector2Array([Vector2(lx - 4.0, shoulder_y + 2.5), Vector2(lx + 4.0, shoulder_y + 2.5), Vector2(lx + 4.0, shoulder_y + 5.5), Vector2(lx - 4.0, shoulder_y + 5.5)]), Color("#5c6570"))
			ItemArt.EMPTY_CUP:
				_shape(PackedVector2Array([hand + Vector2(-1.3, -2.6), hand + Vector2(1.3, -2.6), hand + Vector2(1.1, 0.2), hand + Vector2(-1.1, 0.2)]), Color("#f4f1ea"))
			ItemArt.EMPLOYEE_CARD, ItemArt.GUEST_PASS:
				_shape(PackedVector2Array([hand + Vector2(-1.2, -1.6), hand + Vector2(1.2, -1.6), hand + Vector2(1.2, 0), hand + Vector2(-1.2, 0)]), Color("#f4f6f8") if held == ItemArt.EMPLOYEE_CARD else Color("#f1c40f"))
			ItemArt.FRUIT:
				_blob(hand + Vector2(0, -1), 1.3, Color("#d8452f"))
			_:
				ItemArt.draw(self, held, hand + Vector2(-2.4, -4.0), 0.3)
	match status:
		ACT_BREWING, ACT_COMPUTER:
			# Thinking / typing dots above the head.
			var n := (ms / 300) % 4
			for i in n:
				_blob(Vector2(-3.0 + i * 3.0, top - 3.5), 0.8, Color("#f4ead0") if status == ACT_BREWING else Color("#a8d0f0"))
		ACT_SOFA:
			_draw_z(Vector2(3.5, top - 2.0), ms)
		ACT_TOILET:
			_blob(Vector2(0, top - 3.5), 2.0, Color("#f4f4f4"))
			draw_circle(Vector2(0, top - 3.5), 0.8, Color("#b8bec4"))
		ACT_SMOKING:
			# Cigarette at the mouth; smoke curls up in inked puffs.
			var cx := head.x + (dir * 3.2 if side else 1.4)
			var cy := head.y + 3.4
			draw_line(Vector2(cx, cy), Vector2(cx + (dir if side else 1.0) * 2.6, cy), Color("#f4f1ea"), 0.8)
			draw_circle(Vector2(cx + (dir if side else 1.0) * 2.8, cy), 0.5, Color("#ff7043"))
			for i in 3:
				var k := float((ms + i * 500) % 1500) / 1500.0
				var p := Vector2(cx + 2.5 + sin(k * TAU + i) * 1.5, cy - 1.5 - k * 10.0)
				var r := 0.8 + k * 1.8
				draw_circle(p, r + 0.4, Color(INK, 0.35 * (1.0 - k)))
				draw_circle(p, r, Color(0.82, 0.8, 0.76, 0.75 * (1.0 - k)))
	if status == ACT_WASHING:
		for i in 3:
			var k := float((ms + i * 250) % 750) / 750.0
			var p := Vector2(-3.0 + i * 3.0, hip_y - k * 4.0)
			draw_arc(p, 0.9, 0, TAU, 10, Color(INK, 0.6 * (1.0 - k)), 0.4, true)
			draw_circle(p, 0.8, Color(0.8, 0.92, 1.0, 0.7 * (1.0 - k)))
	if smelly:
		# Green stink lines curling up.
		for i in 3:
			var k := float((ms + i * 400) % 1200) / 1200.0
			var sx := -6.0 + i * 6.0
			var pts := PackedVector2Array()
			for j in 6:
				var y := top + 4.0 - k * 8.0 - j * 1.2
				pts.append(Vector2(sx + sin(j * 1.3 + k * TAU) * 1.0, y))
			draw_polyline(pts, Color(0.45, 0.7, 0.2, 0.8 * (1.0 - k)), 0.7, true)
	if umbrella:
		var cy := top - 3.0
		_limb(Vector2(0, cy), Vector2(0, shoulder_y + 2), 0.5, Color("#5c6570"))
		_shape(_arc_pts(Vector2(0, cy + 1.5), 9.0, PI, TAU, 18), Color("#3a5f9e"))
		for i in 4:
			draw_line(Vector2(0, cy - 7.4), Vector2(-9.0 + i * 6.0, cy + 1.5), Color(INK, 0.6), 0.4, true)
	if slow and (ms / 500) % 2 == 0:
		var d := head + Vector2(HEAD_R - 0.5, -1.5)
		_shape(PackedVector2Array([d + Vector2(0, -1.4), d + Vector2(0.8, 0.2), d + Vector2(0, 0.9), d + Vector2(-0.8, 0.2)]), Color("#9fd8ff"))


func _draw_z(p: Vector2, ms: int) -> void:
	for i in 2:
		var k := float((ms + i * 700) % 1400) / 1400.0
		var q := p + Vector2(k * 2.0, -k * 5.0 - i * 2.0)
		var s := 1.2 + k
		var z := PackedVector2Array([q + Vector2(-s, -s), q + Vector2(s, -s), q + Vector2(-s, s), q + Vector2(s, s)])
		draw_polyline(z, Color(INK, 1.0 - k), 0.9, true)
		draw_polyline(z, Color(1, 1, 1, 0.9 * (1.0 - k)), 0.45, true)
