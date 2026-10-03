## Rolling a cigarette (F with rolling tobacco in hands): three quick
## steps, all with the space bar - fill the paper (hold, let go in the green),
## roll it (press when the marker's in the middle), lick and seal (press at
## "TERAZ!"). The average is the roll's quality (0..100), sent to the server.
extends Control

const Ink = preload("res://ui/ink_ui.gd")

signal rolled(quality: int)

const STEPS := ["Napchaj tytoń: przytrzymaj SPACJĘ i puść na zielonym", "Zwiń bibułkę: SPACJA, gdy znacznik jest na środku",
	"Poliż i sklej: SPACJA, gdy pojawi się TERAZ!"]
## Step 1: the green zone of the fill (0..1) and how fast it fills.
const FILL_FROM := 0.55
const FILL_TO := 0.8
const FILL_SPEED := 0.6
## Step 2: the marker's swing (seconds per sweep).
const ROLL_PERIOD := 1.2
## Step 3: a random wait, then this long to react.
const SEAL_WAIT := Vector2(0.6, 1.6)
const SEAL_WINDOW := 0.6

var step := 0
var scores: Array[int] = []
var _fill := 0.0
var _holding := false
var _t := 0.0
var _seal_at := 0.0
var _done_at := -1.0
var _panel := PanelContainer.new()
var _title := Label.new()
var _hint := Label.new()
var _bar := Control.new()


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	visible = false
	_panel.add_theme_stylebox_override("panel", Ink.box("paper"))
	_panel.custom_minimum_size = Vector2(520, 0)
	add_child(_panel)
	var col := VBoxContainer.new()
	col.add_theme_constant_override("separation", 10)
	_panel.add_child(col)
	Ink.style_label(_title, 22, Ink.TEXT_INK)
	col.add_child(_title)
	Ink.style_label(_hint, 16, Color("#4a5566"))
	_hint.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	col.add_child(_hint)
	_bar.custom_minimum_size = Vector2(480, 46)
	_bar.draw.connect(_draw_bar)
	col.add_child(_bar)
	get_viewport().size_changed.connect(_place)


func _place() -> void:
	var vs := get_viewport_rect().size
	position = Vector2.ZERO
	size = vs
	_panel.reset_size()
	_panel.position = Vector2((vs.x - _panel.size.x) / 2, vs.y * 0.3)


## Open it (the tobacco is in hands).
func start() -> void:
	step = 0
	scores.clear()
	_fill = 0.0
	_holding = false
	_t = 0.0
	_done_at = -1.0
	visible = true
	_show()
	_place.call_deferred()


func cancel() -> void:
	visible = false


## Tests: skip the playing.
func auto_finish(quality: int) -> void:
	visible = false
	rolled.emit(clampi(quality, 0, 100))


## The scores of the three steps (0..100 each).
static func fill_score(fill: float) -> int:
	if fill >= FILL_FROM and fill <= FILL_TO:
		return 100
	var off := FILL_FROM - fill if fill < FILL_FROM else fill - FILL_TO
	return clampi(int(100 - off * 400), 0, 100)


static func roll_score(marker: float) -> int:
	return clampi(int(100 - absf(marker - 0.5) * 300), 0, 100)


static func seal_score(reaction: float) -> int:
	if reaction < 0.0:
		return 20  # too early: a soggy mess
	return clampi(int(100 - reaction * 150), 0, 100)


func _marker() -> float:
	return 0.5 + 0.5 * sin(_t * TAU / ROLL_PERIOD)


func _process(delta: float) -> void:
	if not visible:
		return
	_t += delta
	if _done_at >= 0.0:
		if _t >= _done_at:
			visible = false
		return
	if step == 0 and _holding:
		_fill = minf(_fill + FILL_SPEED * delta, 1.2)
	if step == 2 and _seal_at > 0.0 and _t > _seal_at + SEAL_WINDOW:
		_next(seal_score(SEAL_WINDOW))  # too slow
	_bar.queue_redraw()


func _unhandled_input(event: InputEvent) -> void:
	if not visible or _done_at >= 0.0 or not (event is InputEventKey):
		return
	if event.keycode == KEY_ESCAPE and event.pressed:
		cancel()
		get_viewport().set_input_as_handled()
		return
	if event.keycode != KEY_SPACE or event.echo:
		return
	get_viewport().set_input_as_handled()
	match step:
		0:
			if event.pressed:
				_holding = true
			elif _holding:
				_holding = false
				_next(fill_score(_fill))
		1:
			if event.pressed:
				_next(roll_score(_marker()))
		2:
			if event.pressed:
				_next(seal_score(_t - _seal_at if _t >= _seal_at else -1.0))


func _next(score: int) -> void:
	scores.append(score)
	step += 1
	_t = 0.0
	if step == 2:
		_seal_at = randf_range(SEAL_WAIT.x, SEAL_WAIT.y)
	if step >= STEPS.size():
		var q := 0
		for s in scores:
			q += s
		q = int(round(q / float(scores.size())))
		_title.text = "Skręt gotowy: %d / 100" % q
		_hint.text = "Napchanie %d · zwinięcie %d · sklejenie %d" % scores
		_done_at = 1.4
		_t = 0.0
		rolled.emit(q)
		_bar.queue_redraw()
		return
	_show()


func _show() -> void:
	_title.text = "Skręcanie (%d/3)" % (step + 1)
	_hint.text = STEPS[step] + "  ·  Esc — odłóż"
	_bar.queue_redraw()


func _draw_bar() -> void:
	var r := Rect2(Vector2(0, 8), Vector2(_bar.size.x, 30))
	_bar.draw_rect(r, Color("#f4ead0"))
	match step:
		0:
			_bar.draw_rect(Rect2(r.position + Vector2(r.size.x * FILL_FROM, 0), Vector2(r.size.x * (FILL_TO - FILL_FROM), r.size.y)), Color("#9fd46a"))
			_bar.draw_rect(Rect2(r.position, Vector2(r.size.x * minf(_fill, 1.0), r.size.y)), Color(0.55, 0.38, 0.2, 0.85))
		1:
			_bar.draw_rect(Rect2(r.position + Vector2(r.size.x * 0.45, 0), Vector2(r.size.x * 0.1, r.size.y)), Color("#9fd46a"))
			var x := r.position.x + r.size.x * _marker()
			_bar.draw_rect(Rect2(Vector2(x - 3, r.position.y - 4), Vector2(6, r.size.y + 8)), Ink.INK)
		2:
			var now := _seal_at > 0.0 and _t >= _seal_at
			_bar.draw_rect(r, Color("#9fd46a") if now else Color("#f4ead0"))
			_bar.draw_string(ThemeDB.fallback_font, r.position + Vector2(r.size.x / 2 - 40, 22), "TERAZ!" if now else "czekaj…",
				HORIZONTAL_ALIGNMENT_LEFT, -1, 20, Ink.INK)
	_bar.draw_rect(r, Ink.INK, false, 2.0)
