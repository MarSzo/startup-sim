## The fridge in the kitchenette (E at it): people's food and drinks (take
## any - even someone else's), milk for the coffee, free company drinks, and
## putting what you hold inside.
extends Control

const Ink = preload("res://ui/ink_ui.gd")
const ItemArt = preload("res://game/item_art.gd")
const Protocol = preload("res://net/protocol.gd")

## FridgeAction to send: action, arg (stored item index).
signal action(action: int, arg: int)

var held := 0  # what the player holds (ItemArt kind), for the buttons
var _panel := PanelContainer.new()
var _list := VBoxContainer.new()
var _last := {}


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	visible = false
	_panel.add_theme_stylebox_override("panel", Ink.box("paper"))
	add_child(_panel)
	_list.add_theme_constant_override("separation", 6)
	_panel.add_child(_list)
	_panel.resized.connect(_place)
	get_viewport().size_changed.connect(_place)


func _place() -> void:
	var vs := get_viewport_rect().size
	position = Vector2.ZERO
	size = vs
	_panel.position = Vector2((vs.x - _panel.size.x) / 2, vs.y - _panel.size.y - 110)


func _row(kind: int, text: String, button: String, act: int, arg: int, enabled := true) -> void:
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 10)
	var icon := Control.new()
	icon.custom_minimum_size = Vector2(32, 32)
	icon.draw.connect(func(): if kind != 0: ItemArt.draw(icon, kind, Vector2.ZERO, 2.0))
	row.add_child(icon)
	var l := Ink.label(text, 16, Ink.TEXT_INK)
	l.custom_minimum_size = Vector2(300, 0)
	l.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	row.add_child(l)
	if button != "":
		var b := Ink.button(button, true)
		b.focus_mode = Control.FOCUS_NONE
		b.disabled = not enabled
		b.pressed.connect(func(): action.emit(act, arg))
		row.add_child(b)
	_list.add_child(row)


func show_fridge(p: Dictionary) -> void:
	_last = p
	for c in _list.get_children():
		c.queue_free()
	_list.add_child(Ink.label("Lodówka", 20, Ink.TEXT_INK))
	if p.items.is_empty():
		_list.add_child(Ink.label("Pusto… ktoś wszystko zjadł.", 16, Ink.TEXT_MUTED))
	for i in p.items.size():
		_row(p.items[i].kind, p.items[i].label, "Weź", Protocol.FRIDGE_TAKE, i)
	_list.add_child(Ink.label("Firmowe (za darmo, co rano nowe):", 16, Ink.TEXT_MUTED))
	_row(ItemArt.WATER, "Woda — %d szt." % p.water, "Weź", Protocol.FRIDGE_WATER, 0, p.water > 0)
	_row(ItemArt.JUICE, "Sok pomarańczowy — %d szt." % p.juice, "Weź", Protocol.FRIDGE_JUICE, 0, p.juice > 0)
	_row(ItemArt.MILK, "Mleko do kawy — %d porcji" % p.milk, "Dolej do kawy", Protocol.FRIDGE_MILK, 0, p.milk > 0 and held == ItemArt.COFFEE)
	var put := "Włóż: %s" % ItemArt.item_name(held) if held != 0 else "Włóż (nic nie trzymasz)"
	_row(held, "Do lodówki to, co trzymasz", put, Protocol.FRIDGE_PUT, 0, fridge_worthy(held))
	var hint := Ink.label("Esc zamknij · karton mleka ze sklepu uzupełnia mleko", 16, Ink.TEXT_MUTED)
	_list.add_child(hint)
	visible = true
	_panel.reset_size()
	_place.call_deferred()


## Same rule as the server (kitchen::fridge_worthy): food, drinks, milk.
static func fridge_worthy(k: int) -> bool:
	return (k >= 10 and k <= 22) or (k >= 25 and k <= 33) or k == ItemArt.MILK or k == ItemArt.FRUIT


## What's in hands changed: refresh the buttons.
func set_held(k: int) -> void:
	if k != held:
		held = k
		if visible and not _last.is_empty():
			show_fridge(_last)


func close() -> void:
	visible = false


func _unhandled_input(event: InputEvent) -> void:
	if visible and event is InputEventKey and event.pressed and not event.echo and event.keycode == KEY_ESCAPE:
		close()
		get_viewport().set_input_as_handled()
