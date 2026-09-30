## The company's departments, as the server sends them (`Departments`
## packet): id -> full name and a short one for next to a nick. Shared by
## every screen (static); until the list comes, the ids show as "?".
extends RefCounted

## The board (it doesn't hire): not offered for positions.
const BOARD := 3

static var names := {}   # id -> "Obsługa klienta"
static var shorts := {}  # id -> "Obsługa"


static func set_list(list: Array) -> void:
	names.clear()
	shorts.clear()
	for d in list:
		names[int(d.id)] = d.name
		shorts[int(d.id)] = d.short


static func name_of(id: int, fallback := "?") -> String:
	return names.get(id, fallback)


static func short_of(id: int, fallback := "") -> String:
	return shorts.get(id, fallback)


## Departments a position can be in (all but the board), in id order.
static func for_positions() -> Array:
	var out := []
	for id in names:
		if id != BOARD:
			out.append(id)
	out.sort()
	return out
