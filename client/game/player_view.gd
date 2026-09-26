## Pixel-art character (+ nick label and speech bubble). Position is in world
## px; the node origin is the feet (collision box center).
## Appearance comes from a seed (the entity id) or a fixed look for NPC staff;
## the walk cycle is driven by the distance actually travelled, so it matches
## the movement for both the local player and interpolated remote ones.
extends Node2D

const PixelUI = preload("res://ui/pixel_ui.gd")
const ItemArt = preload("res://game/item_art.gd")

const FACING_DOWN := 0
const FACING_UP := 1
const FACING_LEFT := 2
const FACING_RIGHT := 3
const BUBBLE_WIDTH := 260.0
const HEAD_TOP := -21.0  # sprite top relative to the feet

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
const OUTLINE := Color(0.08, 0.08, 0.1)

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
	ls.font = PixelUI.font()
	ls.font_size = 16
	ls.outline_size = 4
	ls.outline_color = PixelUI.INK
	nick_label.label_settings = ls
	nick_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	nick_label.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	# Render the label at screen resolution regardless of camera zoom.
	nick_label.scale = Vector2.ONE / zoom
	nick_label.size = Vector2(200, 24)
	nick_label.position = Vector2(-100 / zoom, HEAD_TOP - 24 / zoom)
	if nick_label.get_parent() == null:
		add_child(nick_label)
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
	bubble.add_theme_stylebox_override("panel", PixelUI.box("bubble"))
	bubble_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	bubble_label.custom_minimum_size = Vector2(BUBBLE_WIDTH, 0)
	PixelUI.style_label(bubble_label, 16, PixelUI.TEXT_INK)
	bubble.add_child(bubble_label)
	bubble.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	bubble.scale = Vector2.ONE / _zoom
	bubble.visible = false
	bubble.z_index = 10
	add_child(bubble)


func set_nick(nick: String) -> void:
	nick_label.text = nick


## Show a speech bubble above the head for a few seconds.
func say(text: String) -> void:
	bubble_label.text = text
	bubble.reset_size()
	bubble.visible = true
	_bubble_time = 3.0 + text.length() * 0.05
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


func _process(delta: float) -> void:
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

func _r(x: float, y: float, w: float, h: float, c: Color) -> void:
	draw_rect(Rect2(x, y, w, h), c)


func _draw() -> void:
	var f := _frame()
	var step: int = [0, 1, 0, -1][f]    # leg swing
	var bob := 1 if f % 2 == 1 else 0     # body bounces while walking
	var side := facing == FACING_LEFT or facing == FACING_RIGHT
	var dir := -1 if facing == FACING_LEFT else 1

	# Shadow.
	draw_rect(Rect2(-5, 1, 10, 3), Color(0, 0, 0, 0.28))
	draw_rect(Rect2(-4, 0, 8, 1), Color(0, 0, 0, 0.18))

	# Sitting (sofa, toilet, computer): lower body, short legs.
	var sit := status in [ACT_SOFA, ACT_TOILET, ACT_COMPUTER] and _idle >= 0.12
	var top := HEAD_TOP + bob + (3 if sit else 0)  # head top
	# Outline silhouette first (1px bigger), then the parts.
	var ol := OUTLINE if not highlight else Color(1, 1, 1, 0.95)
	_r(-4, top - 1, 8, 9, ol)           # head
	_r(-5, top + 7, 10, 9, ol)          # torso + arms
	_r(-4, top + 15, 8, (3 if sit else 6) - bob, ol)    # legs

	# Legs + shoes.
	var leg_y := top + 15
	var leg_h := (2 if sit else 5) - bob
	if side:
		_r(-2 + step, leg_y, 3, leg_h, pants)
		_r(-1 - step, leg_y, 3, leg_h, pants.darkened(0.2))
		_r(-2 + step + (1 if dir > 0 else -1), leg_y + leg_h - 1, 3, 1, Color("#1c1c1c"))
	else:
		var l_lift := 1 if step > 0 else 0
		var r_lift := 1 if step < 0 else 0
		_r(-3, leg_y, 3, leg_h - l_lift, pants)
		_r(0, leg_y, 3, leg_h - r_lift, pants.darkened(0.12))
		_r(-3, leg_y + leg_h - 1 - l_lift, 3, 1, Color("#1c1c1c"))
		_r(0, leg_y + leg_h - 1 - r_lift, 3, 1, Color("#1c1c1c"))

	# Torso and arms (arms swing opposite to the legs).
	var ty := top + 8
	_r(-4, ty, 8, 7, shirt)
	_r(-4, ty, 8, 1, shirt.lightened(0.15))
	if look == LOOK_OFFICE and facing != FACING_UP:
		_r(-0.5 if not side else dir * 1.5 - 0.5, ty + 1, 1.5, 5, tie)
	if look == LOOK_FIREFIGHTER:
		_r(-4, ty + 2, 8, 1, Color("#f1e05a"))  # reflective stripes
		_r(-4, ty + 5, 8, 1, Color("#c9d1d9"))
	if look == LOOK_CLEANER:
		if facing != FACING_UP:
			_r(-3, ty + 2, 6, 5, Color("#e8f4f2"))  # apron
		# Mop: handle at her side, head on the floor.
		var mx := 6 if not side else dir * 5
		_r(mx, ty - 2, 1, 14, Color("#a0764b"))
		_r(mx - 2, ty + 12, 5, 2, Color("#d9d4c7"))
	if look == LOOK_GUARD:
		_r(-4, ty + 2, 8, 2, Color("#f1c40f"))  # "OCHRONA" band
		if facing != FACING_UP:
			_r(-2, ty + 2, 4, 1, Color("#23262b"))
	if look == LOOK_POLICE:
		_r(-4, ty + 5, 8, 1, Color("#101010"))  # belt
		if facing != FACING_UP and not side:
			_r(1, ty + 1, 2, 2, Color("#d9d9d9"))  # badge
		_r(-5 if not side else -1, ty + 1, 2, 1, Color("#d4ac2b"))  # epaulette
	if look == LOOK_PORTER:
		_r(-4, ty + 5, 8, 1, Color("#d4ac2b"))  # belt
		if facing != FACING_UP and not side:
			_r(1, ty + 1, 2, 2, Color("#d4ac2b"))  # badge
	var swing: int = -step
	if side:
		_r(-1 + swing * dir, ty + 1, 3, 5, shirt.darkened(0.18))
		_r(-1 + swing * dir, ty + 6, 2, 2, skin)
	else:
		_r(-5, ty + 1 + maxi(swing, 0), 2, 5, shirt.darkened(0.18))
		_r(3, ty + 1 + maxi(-swing, 0), 2, 5, shirt.darkened(0.18))
		_r(-5, ty + 6 + maxi(swing, 0), 2, 1, skin)
		_r(3, ty + 6 + maxi(-swing, 0), 2, 1, skin)

	# Head.
	_r(-3, top, 6, 8, skin)
	_r(-3, top + 7, 6, 1, skin.darkened(0.12))
	_draw_hair(top, side, dir)
	# Face.
	var eye := Color("#1f1f24")
	match facing:
		FACING_DOWN:
			_r(-2, top + 4, 1, 1, eye)
			_r(1, top + 4, 1, 1, eye)
			_r(-1, top + 6, 2, 1, skin.darkened(0.25))
		FACING_LEFT, FACING_RIGHT:
			_r(dir * 1.5 - 0.5, top + 4, 1, 1, eye)
			_r(dir * 3 - (1 if dir > 0 else 0), top + 5, 1, 1, skin.darkened(0.15))  # nose
	_draw_status(top, ty, side, dir)
	if look == LOOK_FIREFIGHTER:
		_r(-4, top - 2, 8, 4, Color("#d62f2f"))           # helmet
		_r(-5, top + 1, 10, 1, Color("#a31f1f"))          # brim
		_r(-1, top - 2, 2, 1, Color("#f1e05a"))            # badge
	if look == LOOK_POLICE:
		_r(-4, top - 1, 8, 3, Color("#17233d"))           # cap
		_r(-4, top + 1, 8, 1, Color("#e8e8e8"))           # band
		for i in 4:
			_r(-4 + i * 2, top + 1, 1, 1, Color("#17233d"))
		if facing == FACING_DOWN:
			_r(-4, top + 2, 8, 1, Color("#0b0f1a"))        # visor
	if look == LOOK_PORTER:
		_r(-4, top - 1, 8, 3, Color("#1b2440"))           # cap
		_r(-2, top, 4, 1, Color("#d4ac2b"))                # cap badge
		if facing == FACING_DOWN:
			_r(-4, top + 2, 8, 1, Color("#10162a"))        # visor
		elif side:
			_r(dir * 2 - (2 if dir < 0 else 0) + (1 if dir > 0 else -1), top + 2, 3, 1, Color("#10162a"))


func _draw_status(top: float, ty: float, side: bool, dir: int) -> void:
	if held != 0 and facing != FACING_UP:
		var mx := (dir * 4.0 - 1.0) if side else 3.0
		match held:
			ItemArt.COFFEE:
				# Mug in the right hand (+ a wisp of steam).
				_r(mx, ty + 4, 3, 3, Color("#f4f1ea"))
				_r(mx, ty + 4, 3, 1, Color("#6b4a2e"))
				_r(mx + 3, ty + 5, 1, 1, Color("#d9d4c8"))
				var t := Time.get_ticks_msec() / 400
				_r(mx + (t % 2), ty + 2, 1, 1, Color(1, 1, 1, 0.7))
				_r(mx + 1 - (t % 2), ty + 1, 1, 1, Color(1, 1, 1, 0.45))
				if t % 3 == 0:
					queue_redraw()
			ItemArt.LAPTOP:
				# Laptop carried in front / under the arm.
				var lx := (dir * 2.0 - 3.0) if side else -4.0
				_r(lx, ty + 3, 8, 5, Color("#5c6570"))
				_r(lx, ty + 3, 8, 1, Color("#8a939c"))
			ItemArt.EMPLOYEE_CARD:
				_r(mx, ty + 5, 3, 2, Color("#f4f6f8"))
				_r(mx, ty + 5, 3, 1, Color("#2e6bd9"))
			ItemArt.GUEST_PASS:
				_r(mx, ty + 5, 3, 2, Color("#f1c40f"))
			ItemArt.FRUIT:
				_r(mx, ty + 4, 3, 3, Color("#e74c3c"))
				_r(mx + 1, ty + 3, 1, 1, Color("#27ae60"))
			_:
				# Shop goods: the item's icon, small, in the hand.
				ItemArt.draw(self, held, Vector2(mx - 1, ty + 2), 0.3)
	var ms := Time.get_ticks_msec()
	match status:
		ACT_BREWING, ACT_COMPUTER:
			# "Brewing…" / typing dots above the head (blue at the computer).
			var n := (ms / 300) % 4
			var dc := Color(1, 1, 1, 0.9) if status == ACT_BREWING else Color("#8fc4ff")
			for i in n:
				_r(-4 + i * 3, top - 5, 2, 2, dc)
		ACT_SOFA:
			# Floating "z".
			var zy := top - 5 - float((ms / 250) % 6) * 0.5
			var zc := Color(1, 1, 1, 0.85)
			_r(3, zy, 3, 1, zc)
			_r(4, zy + 1, 1, 1, zc)
			_r(3, zy + 2, 3, 1, zc)
		ACT_TOILET:
			# A roll of toilet paper above the head.
			_r(-2, top - 7, 4, 4, Color("#f4f4f4"))
			_r(-1, top - 6, 2, 2, Color("#b8bec4"))
		ACT_SMOKING:
			# Cigarette + smoke puffs rising.
			var cx := (dir * 3.0) if side else 1.0
			_r(cx, top + 6, 3, 1, Color("#f4f1ea"))
			_r(cx + (2 if dir > 0 or not side else 0), top + 6, 1, 1, Color("#ff7043"))
			var k := float(ms % 1200) / 1200.0
			_r(cx + 1 + k * 2, top + 3 - k * 6, 2, 2, Color(0.8, 0.8, 0.8, 0.7 * (1.0 - k)))
			_r(cx + 2 - k, top - 1 - k * 5, 2, 2, Color(0.8, 0.8, 0.8, 0.5 * (1.0 - k)))
	if status == ACT_WASHING:
		# Water and soap bubbles at the hands.
		for i in 3:
			var k := float((ms + i * 250) % 750) / 750.0
			_r(-4 + i * 3, ty + 7 - k * 4, 2, 2, Color(0.75, 0.9, 1.0, 0.9 * (1.0 - k)))
	if smelly:
		# Wavy green fumes rising around the head.
		for i in 3:
			var k := float((ms + i * 400) % 1200) / 1200.0
			var sx := -6.0 + i * 5.0 + sin(k * TAU + i) * 1.5
			_r(sx, top + 2 - k * 10, 2, 2, Color(0.45, 0.75, 0.2, 0.75 * (1.0 - k)))
	if umbrella:
		# Open umbrella: canopy with ribs over the head, shaft to the hand.
		var cy := top - 6
		draw_rect(Rect2(-1, cy, 1, 12), Color("#5c6570"))
		var canopy := PackedVector2Array([Vector2(-10, cy + 2), Vector2(0, cy - 5), Vector2(10, cy + 2)])
		draw_colored_polygon(canopy, Color("#2e6bd9"))
		for i in 4:
			draw_rect(Rect2(-10 + i * 5, cy + 1, 5, 2), Color("#2e6bd9") if i % 2 == 0 else Color("#8fb7ff"))
		draw_rect(Rect2(-1, cy - 6, 1, 1), Color("#1c1c24"))
	if slow:
		# A drop of sweat.
		if (ms / 500) % 2 == 0:
			_r(4, top + 1, 1, 2, Color("#7fd3ff"))


func _draw_hair(top: float, side: bool, dir: int) -> void:
	if look == LOOK_PORTER or look == LOOK_POLICE or look == LOOK_FIREFIGHTER:
		_r(-3, top + 1, 6, 2, hair)  # a bit of hair under the cap
		return
	var h := hair
	var hl := hair.lightened(0.18)
	if hair_style == 5:  # bald: a hint of shine only
		_r(-2, top, 3, 1, skin.lightened(0.25))
		return
	match facing:
		FACING_UP:
			_r(-3, top, 6, 7 if hair_style != 1 else 8, h)
			_r(-2, top, 3, 1, hl)
			if hair_style == 1:
				_r(-3, top + 7, 6, 3, h)
			elif hair_style == 4:
				_r(-1, top + 7, 2, 4, h)
			elif hair_style == 2:
				_r(-1, top - 2, 3, 2, h)
		_:
			_r(-3, top, 6, 3, h)
			_r(-2, top, 3, 1, hl)
			match hair_style:
				1:  # long
					if side:
						_r(-3 if dir > 0 else 1, top + 2, 2, 7, h)
					else:
						_r(-4, top + 1, 2, 8, h)
						_r(2, top + 1, 2, 8, h)
				2:  # bun
					_r(-1, top - 3, 3, 3, h)
				3:  # spiky
					for i in 3:
						_r(-3 + i * 2, top - 1, 1, 1, h)
				4:  # ponytail
					if side:
						_r(-4 if dir > 0 else 2, top + 2, 2, 5, h)
					else:
						_r(3, top + 2, 2, 4, h)
			if side:
				_r(-3 if dir > 0 else 1, top, 2, 5, h)  # back of the head
