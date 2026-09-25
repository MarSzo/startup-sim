## Headless tests: protocol + movement parity with the Rust server.
## Run: godot --headless --path client -s tests/run_tests.gd
extends SceneTree

const Protocol = preload("res://net/protocol.gd")
const Movement = preload("res://sim/movement.gd")
const MapData = preload("res://map/map_data.gd")

var failures := 0
var checks := 0


func _init() -> void:
	var golden := ProjectSettings.globalize_path("res://").path_join("../server/tests/golden")
	test_protocol(golden.path_join("packets.json"))
	test_movement(golden.path_join("movement_vectors.json"))
	test_rejects_garbage()
	print("%d checks, %d failures" % [checks, failures])
	quit(1 if failures > 0 else 0)


func expect(cond: bool, msg: String) -> void:
	checks += 1
	if not cond:
		failures += 1
		printerr("FAIL: ", msg)


func load_json(path: String):
	var text := FileAccess.get_file_as_string(path)
	expect(text != "", "read " + path)
	return JSON.parse_string(text)


func test_protocol(path: String) -> void:
	var golden := {}
	for item in load_json(path)["packets"]:
		golden[item["name"]] = item["hex"]
	# Mirrors protocol::golden_samples() in server/src/protocol.rs.
	var enc := {
		"connect": Protocol.encode_connect(0xDEADBEEF, "Zażółć"),
		"input": Protocol.encode_input(0x01020304, 1200, 99, PackedByteArray([0, 1, 9, 6])),
		"info_request": Protocol.encode_info_request(0x01020304, [3, 4, 500]),
		"ping": Protocol.encode_ping(0x01020304, 777000),
	}
	for name in enc:
		expect(enc[name].hex_encode() == golden[name], "encode %s: %s != %s" % [name, enc[name].hex_encode(), golden[name]])

	var w := Protocol.decode(golden["welcome"].hex_decode())
	expect(w.get("type") == Protocol.T_WELCOME and w.nonce == 0xDEADBEEF and w.player_id == 7 and w.token == 0x01020304
		and w.tick_hz == 20 and w.input_hz == 60 and w.map_crc == 0xCAFEBABE and w.server_tick == 1234, "decode welcome %s" % w)
	var rj := Protocol.decode(golden["reject"].hex_decode())
	expect(rj.get("reason") == 1, "decode reject")
	var s := Protocol.decode(golden["snapshot"].hex_decode())
	expect(s.get("tick") == 1234 and s.last_input_seq == 99 and s.frag_cnt == 1 and s.self_x == 10000 and s.self_y == -5
		and s.room == 6 and s.entities.size() == 2, "decode snapshot %s" % s)
	if s.has("entities") and s.entities.size() == 2:
		var e0: Dictionary = s.entities[0]
		var e1: Dictionary = s.entities[1]
		expect(e0.id == 3 and e0.kind == 0 and e0.x == 4096 and e0.y == 8192 and e0.flags == 5, "entity 0 %s" % e0)
		expect(e1.id == 65535 and e1.kind == 1 and e1.x == -1 and e1.y == 2000000, "entity 1 %s" % e1)
	var pi := Protocol.decode(golden["player_info"].hex_decode())
	expect(pi.get("players", []).size() == 2 and pi.players[0].nick == "Ala" and pi.players[1].nick == "bot_07" and pi.players[1].id == 4, "decode player_info %s" % pi)
	var po := Protocol.decode(golden["pong"].hex_decode())
	expect(po.get("client_time") == 777000 and po.server_tick == 1234, "decode pong")
	var d := Protocol.decode(golden["disconnect"].hex_decode())
	expect(d.get("reason") == 1 and d.token == 0x01020304, "decode disconnect")
	# Truncation must never decode.
	var snap: PackedByteArray = golden["snapshot"].hex_decode()
	for n in snap.size():
		expect(Protocol.decode(snap.slice(0, n)).is_empty(), "truncated snapshot len %d" % n)
	# Nick truncated on a character boundary (20 bytes -> 16).
	var c := Protocol.encode_connect(1, "ąąąąąąąąąą")
	expect(c[8] == 16 and c.size() == 9 + 16, "nick truncation")


func test_movement(path: String) -> void:
	var map = MapData.new()
	map.load_path("res://maps/floor0.json")
	expect(map.error == "", "map loads: " + map.error)
	var data = load_json(path)
	expect(int(data["map_crc"]) == map.crc, "map crc parity %d vs %d" % [int(data["map_crc"]), map.crc])
	var case_i := 0
	for c in data["cases"]:
		var p := Vector2i(int(c["start"][0]), int(c["start"][1]))
		var inputs: Array = c["inputs"]
		var positions: Array = c["positions"]
		var ok := true
		for i in inputs.size():
			p = Movement.step(map, p, int(inputs[i]))
			var want := Vector2i(int(positions[i][0]), int(positions[i][1]))
			if p != want:
				expect(false, "case %d step %d: got %s want %s" % [case_i, i, p, want])
				ok = false
				break
		expect(ok, "movement case %d" % case_i)
		case_i += 1


func test_rejects_garbage() -> void:
	expect(Protocol.decode(PackedByteArray()).is_empty(), "empty")
	expect(Protocol.decode(PackedByteArray([0, 0, 1, 2])).is_empty(), "bad magic")
	expect(Protocol.decode(PackedByteArray([0x54, 0x53, 9, 2])).is_empty(), "bad version")
	expect(Protocol.decode(PackedByteArray([0x54, 0x53, 1, 200])).is_empty(), "unknown type")
