## Notifications in the corner (under the stats): a paper card per event -
## new mail, treats in the chill room, somebody knocked out next to you...
## Each stays a few seconds; at most a few at once (the oldest go first).
extends VBoxContainer

const Ink = preload("res://ui/ink_ui.gd")

const SHOW_SEC := 7.0
const MAX_SHOWN := 4
## Notice.icon -> a symbol.
const ICONS := {1: "ℹ", 2: "✉", 3: "🍽", 4: "⚠", 5: "★"}

var history: Array[String] = []  # every notice of the session (tests, the log)


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_theme_constant_override("separation", 6)
	custom_minimum_size = Vector2(360, 0)
	get_viewport().size_changed.connect(_place)
	_place()


## Top right, under the stats (a CanvasLayer has no anchors to lean on).
func _place() -> void:
	position = Vector2(get_viewport_rect().size.x - 380, 112)


func push(icon: int, text: String) -> void:
	history.append(text)
	var card := PanelContainer.new()
	card.add_theme_stylebox_override("panel", Ink.box("bubble"))
	card.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var l := Label.new()
	Ink.style_label(l, 16, Color("#c0392b") if icon == 4 else Ink.TEXT_INK)
	l.text = "%s  %s" % [ICONS.get(icon, "•"), text]
	l.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	l.custom_minimum_size = Vector2(340, 0)
	card.add_child(l)
	add_child(card)
	while get_child_count() > MAX_SHOWN:
		var old := get_child(0)
		remove_child(old)
		old.queue_free()
	get_tree().create_timer(SHOW_SEC).timeout.connect(func():
		if is_instance_valid(card):
			card.queue_free())
