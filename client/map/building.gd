## The whole building: every floor listed in maps/building.json.
## Mirror of server/src/building.rs.
extends RefCounted

const MapData = preload("res://map/map_data.gd")

## Indexed by floor number: {name, locked, map (MapData or null)}.
var floors: Array = []
## CRC32 over building.json followed by every floor file, in floor order
## (same as the server's; compared with Welcome.map_crc).
var crc := 0
## Elevator ids in the server's order (floors ascending, links in file
## order, first appearance): index of a lift in the Doors packet.
var lift_ids: Array[String] = []
var error := ""


func load_path(path: String) -> void:
	var bytes := FileAccess.get_file_as_bytes(path)
	if bytes.is_empty():
		error = "cannot read %s" % path
		return
	var all := bytes.duplicate()
	var data = JSON.parse_string(bytes.get_string_from_utf8())
	if typeof(data) != TYPE_DICTIONARY:
		error = "invalid building json"
		return
	var dir := path.get_base_dir()
	for f in data["floors"]:
		var entry := {"name": f["name"], "locked": f.get("locked", false), "map": null}
		if f.get("file") != null:
			var fb := FileAccess.get_file_as_bytes(dir.path_join(f["file"]))
			if fb.is_empty():
				error = "cannot read %s" % f["file"]
				return
			all.append_array(fb)
			var m = MapData.new()
			m.parse(fb)
			if m.error != "":
				error = "%s: %s" % [f["file"], m.error]
				return
			entry.map = m
		floors.append(entry)
	crc = MapData.crc32(all)
	for f in floors.size():
		var m = get_floor(f)
		if m == null:
			continue
		for l in m.links:
			if l.kind == "elevator" and not lift_ids.has(l.id):
				lift_ids.append(l.id)


## Map of an active (existing, unlocked) floor, or null.
func get_floor(f: int):
	if f < 0 or f >= floors.size() or floors[f].locked:
		return null
	return floors[f].map


func floor_name(f: int) -> String:
	return floors[f].name if f >= 0 and f < floors.size() else "?"


## Active floors with a cabin of elevator `id` (where it stops), bottom up.
func elevator_floors(id: String) -> Array[int]:
	var out: Array[int] = []
	for f in floors.size():
		var m = get_floor(f)
		if m == null:
			continue
		for l in m.links:
			if l.kind == "elevator" and l.id == id:
				out.append(f)
				break
	return out


## Which elevator a door tile belongs to (it touches that cabin), or -1.
func lift_at_door(f: int, t: Vector2i) -> int:
	var m = get_floor(f)
	if m == null:
		return -1
	for l in m.links:
		if l.kind == "elevator" and (l.rect as Rect2i).grow(1).has_point(t):
			return lift_ids.find(l.id)
	return -1


## Room name on a floor.
func room_name(f: int, rid: int) -> String:
	var m = get_floor(f)
	return m.room_name(rid) if m else "-"
