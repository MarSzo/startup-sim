## Base of an end-to-end scenario: the real client, driven by a script
## (`--scenario=<name>`, see tests/e2e/run.sh). A scenario overrides `run()`
## and uses the helpers below; it ends with "E2E PASS <name>" and exit code
## 0, or "E2E FAIL <name>: why" and exit code 1. Not in exported builds
## (tests/ is excluded).
extends Node

const Movement = preload("res://sim/movement.gd")
const MapData = preload("res://map/map_data.gd")
const Protocol = preload("res://net/protocol.gd")

## The whole run must finish in this many seconds.
const DEADLINE := 240.0

var main            # main.gd
var scenario_name := ""
var _path: Array = []       # [floor, Vector2i] still to walk
var _press := 0             # input bits to send once
var _stuck := 0.0
var _last_pos := Vector2i.ZERO
var _finished := false
var says: Array[String] = []     # every line any NPC / player said near us
var chats: Array[String] = []    # messenger messages seen ("nick: text")
var last: Dictionary = {}        # latest packet of each type (Stats, Clock, Inventory, ...)


func _ready() -> void:
	main.net.packet_received.connect(_on_packet)
	get_tree().create_timer(DEADLINE).timeout.connect(func(): fail("over %d s" % int(DEADLINE)))
	_start.call_deferred()


func _start() -> void:
	log_step("start")
	await run()
	if not _finished:
		done()


## The scenario itself.
func run() -> void:
	pass


# ------------------------------------------------------------------ results

func log_step(text: String) -> void:
	print("E2E %s: %s" % [scenario_name, text])


func done() -> void:
	if _finished:
		return
	_finished = true
	print("E2E PASS %s" % scenario_name)
	main.get_tree().quit(0)


func fail(why: String) -> void:
	if _finished:
		return
	_finished = true
	var where := ""
	if game() and game().have_state:
		var p: Dictionary = game().pred
		where = " (floor %d tile %s, room '%s')" % [p.floor, Movement.tile_of_pos(p.pos), room_name()]
	printerr("E2E FAIL %s: %s%s" % [scenario_name, why, where])
	main.get_tree().quit(1)


## Assert: fail the scenario with `what` unless `cond`.
func check(cond: bool, what: String) -> bool:
	if not cond:
		fail(what)
	return cond


# ------------------------------------------------------------------ state

func game():
	return main.game


func in_world() -> bool:
	return game() != null and game().have_state


func room_name() -> String:
	var g = game()
	if g == null or not g.have_state:
		return ""
	var m = g.building.get_floor(g.pred.floor)
	return m.room_name(g.room_id) if m else ""


func floor_now() -> int:
	return game().pred.floor if in_world() else -1


func tile_now() -> Vector2i:
	return Movement.tile_of_pos(game().pred.pos)


func access() -> int:
	return game().pred.access


## Item kinds in hands (index 0) and pockets.
func items() -> Array:
	return game().inventory.map(func(s): return s.kind) if in_world() else []


func holding(kind: int) -> bool:
	var it := items()
	return not it.is_empty() and it[0] == kind


func carrying(kind: int) -> bool:
	return items().has(kind)


func money() -> int:
	return int(last.get(Protocol.T_STATS, {}).get("money", -1))


func heard(prefix: String) -> bool:
	for s in says:
		if s.begins_with(prefix):
			return true
	return false


func _on_packet(p: Dictionary) -> void:
	last[p.type] = p
	if p.type == Protocol.T_SAY:
		says.append(str(p.text))
	elif p.type == Protocol.T_CHAT:
		for m in p.messages:
			var line := "%s: %s" % [m.nick, m.text]
			if not chats.has(line):
				chats.append(line)


# ------------------------------------------------------------------ waiting

func wait(seconds: float) -> void:
	await get_tree().create_timer(seconds).timeout


## Wait until `cond` holds (checked every frame); false (and the scenario
## failed) after `timeout` seconds.
func until(cond: Callable, timeout: float, what: String) -> bool:
	var left := timeout
	while not cond.call():
		if _finished:
			return false
		await get_tree().process_frame
		left -= get_process_delta_time()
		if left <= 0.0:
			fail("timed out waiting for: " + what)
			return false
	return true


## A messenger message containing `text` arrived (in an open conversation).
func got_chat(text: String) -> bool:
	return chats.any(func(c): return c.contains(text))


## The other player `nick` is in sight (same room).
func sees(nick: String) -> bool:
	var g = game()
	for id in g.remotes:
		if g.nicks.get(id, "") == nick:
			return true
	return false


## The computer screen knows a conversation (e.g. "Kuba" for a DM): the
## messenger commands only work once the server has sent the list.
func messenger_has(title: String) -> bool:
	var st: Dictionary = game().screen.state
	return st.has("convs") and st.convs.any(func(c): return c.title == title)


## Put the laptop on the desk in front and sit down at it.
func sit_at_desk() -> bool:
	await press_e()
	if not await hear("Laptop na biurku"):
		return false
	await press_e()
	return await until(func(): return game().screen.visible, 5.0, "the computer screen")


## Wait until somebody says a line starting with `prefix`.
func hear(prefix: String, timeout := 10.0) -> bool:
	return await until(func(): return heard(prefix), timeout, "„%s”" % prefix)


# ------------------------------------------------------------------ acting

## The game asks for input every physics step (game.script_driver).
func next_input(_delta: float) -> int:
	if _press != 0:
		var b := _press
		_press = 0
		return b
	var g = game()
	while not _path.is_empty() and _path[0][0] != g.pred.floor:
		_path.pop_front()  # the stairs took us to the next floor
	while not _path.is_empty():
		var c := Movement.tile_center(_path[0][1].x, _path[0][1].y)
		var p: Vector2i = g.pred.pos
		var b := 0
		if c.x - p.x > Movement.SPEED / 2: b |= Movement.IN_RIGHT
		elif c.x - p.x < -Movement.SPEED / 2: b |= Movement.IN_LEFT
		if c.y - p.y > Movement.SPEED / 2: b |= Movement.IN_DOWN
		elif c.y - p.y < -Movement.SPEED / 2: b |= Movement.IN_UP
		if b != 0:
			return b
		_path.pop_front()
	return 0


func press_e() -> void:
	_press = Movement.IN_INTERACT
	await wait(0.4)


## Walk to a tile (any floor; stairs on the way), like a player would.
func walk(floor: int, x: int, y: int, timeout := 60.0) -> bool:
	if not await until(in_world, 30.0, "being in the world"):
		return false
	var goal := Vector2i(x, y)
	var path := find_path([floor_now(), tile_now()], [floor, goal], access())
	if path.is_empty():
		fail("no way to floor %d %s" % [floor, goal])
		return false
	_path = path
	_stuck = 0.0
	var left := timeout
	while not _path.is_empty():
		await get_tree().process_frame
		var dt := get_process_delta_time()
		left -= dt
		var now := tile_now()
		_stuck = 0.0 if now != _last_pos else _stuck + dt
		_last_pos = now
		if left <= 0.0 or _stuck > 8.0:
			_path.clear()
			fail("walking to floor %d %s: %s" % [floor, goal, "too slow" if left <= 0.0 else "stuck"])
			return false
	await wait(0.2)
	return check(floor_now() == floor and tile_now() == goal, "arrived at floor %d %s (at floor %d %s)" % [floor, goal, floor_now(), tile_now()])


## Breadth-first over tiles and stairs (the server's Building::find_path).
func find_path(from: Array, to: Array, rights: int) -> Array:
	var b = game().building
	var key := func(f: int, t: Vector2i) -> String: return "%d:%d:%d" % [f, t.x, t.y]
	var prev := {key.call(from[0], from[1]): null}
	var queue: Array = [from]
	var dirs := [[Vector2i(1, 0), MapData.DIR_RIGHT], [Vector2i(-1, 0), MapData.DIR_LEFT],
		[Vector2i(0, 1), MapData.DIR_DOWN], [Vector2i(0, -1), MapData.DIR_UP]]
	while not queue.is_empty():
		var cur: Array = queue.pop_front()
		if cur[0] == to[0] and cur[1] == to[1]:
			var path: Array = []
			var n = cur
			while n != null:
				path.push_front(n)
				n = prev[key.call(n[0], n[1])]
			return path.slice(1)
		var m = b.get_floor(cur[0])
		if m == null:
			continue
		for d in dirs:
			var nt: Vector2i = cur[1] + d[0]
			if m.blocks(nt.x, nt.y, rights, d[1]):
				continue
			var next := [cur[0], nt]
			var link: Dictionary = m.link_at(nt.x, nt.y)
			if not link.is_empty() and link.kind == "stairs" and b.get_floor(link.to_floor) != null:
				# Step onto the flight; the next node is the arrival.
				var k: String = key.call(cur[0], nt)
				if not prev.has(k):
					prev[k] = cur
				var arrival := [link.to_floor, link.to]
				if not prev.has(key.call(arrival[0], arrival[1])):
					prev[key.call(arrival[0], arrival[1])] = [cur[0], nt]
					queue.append(arrival)
				continue
			if not prev.has(key.call(next[0], next[1])):
				prev[key.call(next[0], next[1])] = cur
				queue.append(next)
	return []


## A desk of the given department (the first one), and where to stand.
func desk_of(dept: int) -> Array:
	var b = game().building
	for f in b.floors.size():
		var m = b.get_floor(f)
		if m == null:
			continue
		for y in m.height:
			for x in m.width:
				if m.legend.get(m.tile_chars[y * m.width + x], {}).get("type") == "desk" \
						and m.room_department.get(m.room_at_tile(x, y), 0) == dept and not m.is_blocked(x, y + 1):
					return [f, Vector2i(x, y + 1)]
	return []


## The computer screen's dev commands (see ComputerScreen.dev_command).
func pc(command: String) -> void:
	game().screen.dev_command(command)
	await wait(0.6)


## Hands / pockets: take out pocket `slot`, use / drop / give what's in hands.
func item(action: int, slot := 0) -> void:
	game()._item_action(action, slot)
	await wait(0.4)
