## Headless tests: protocol + movement parity with the Rust server.
## Run: godot --headless --path client -s tests/run_tests.gd
extends SceneTree

const Protocol = preload("res://net/protocol.gd")
const Movement = preload("res://sim/movement.gd")
const MapData = preload("res://map/map_data.gd")
const Building = preload("res://map/building.gd")
const NetClient = preload("res://net/net_client.gd")

var failures := 0
var checks := 0


func _init() -> void:
	var golden := ProjectSettings.globalize_path("res://").path_join("../server/tests/golden")
	test_protocol(golden.path_join("packets.json"))
	test_movement(golden.path_join("movement_vectors.json"))
	test_rejects_garbage()
	test_parse_address()
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
		"connect": Protocol.encode_connect(0xDEADBEEF, "Zażółć", {"gender": 0, "age": 27, "city": "Łódź", "email": "ola@poczta.pl",
			"appearance": {"skin": 1, "hair_style": 4, "hair_color": 2, "shirt": 9, "pants": 3}}),
		"input": Protocol.encode_input(0x01020304, 1200, 99, PackedByteArray([0, 1, 9, 6])),
		"info_request": Protocol.encode_info_request(0x01020304, [3, 4, 500]),
		"ping": Protocol.encode_ping(0x01020304, 777000),
		"apply": Protocol.encode_apply(0x01020304, 2, "Lubię kawę i wyzwania."),
		"portal_action": Protocol.encode_portal_action(0x01020304, Protocol.PORTAL_GO_TO_OFFICE, 0),
		"item_action": Protocol.encode_item_action(0x01020304, Protocol.ITEM_TAKE_OUT, 2),
		"computer_action": Protocol.encode_computer_action(0x01020304, Protocol.PC_SEND, 17, 42, "Kto zjadł mój jogurt?"),
		"answer": Protocol.encode_answer(0x01020304, 3, 1, 2),
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
		and s.floor == 1 and s.room == 6 and s.self_lock == 2 and s.self_prev_input == 17 and s.self_access == 5
		and s.self_status == 1
		and s.entities.size() == 2,
		"decode snapshot %s" % s)
	if s.has("entities") and s.entities.size() == 2:
		var e0: Dictionary = s.entities[0]
		var e1: Dictionary = s.entities[1]
		expect(e0.id == 3 and e0.kind == 0 and e0.x == 4096 and e0.y == 8192 and e0.flags == 5 and e0.held == 3, "entity 0 %s" % e0)
		expect(e1.id == 65535 and e1.kind == 1 and e1.x == -1 and e1.y == 2000000, "entity 1 %s" % e1)
	var pi := Protocol.decode(golden["player_info"].hex_decode())
	expect(pi.get("players", []).size() == 2 and pi.players[0].nick == "Ala" and pi.players[0].department == 1
		and pi.players[0].gender == 0 and pi.players[0].appearance.hair_style == 1 and pi.players[0].appearance.hair_color == 3
		and pi.players[1].nick == "bot_07" and pi.players[1].id == 4 and pi.players[1].department == 0, "decode player_info %s" % pi)
	var jo := Protocol.decode(golden["job_offers"].hex_decode())
	expect(jo.get("offers", []).size() == 2 and jo.offers[1].title == "Dostawca/Dostawczyni" and jo.offers[1].department == 0
		and jo.offers[1].company == "Pizzeria u Stefana" and jo.offers[0].applied == true and jo.offers[1].applied == false
		and jo.offers[0].description == "Owocowe czwartki.", "decode job_offers %s" % jo)
	var inv := Protocol.decode(golden["inventory"].hex_decode())
	expect(inv.get("slots", []).size() == 4 and inv.slots[0].kind == 3 and inv.slots[0].label == "Laptop: Ola"
		and inv.slots[1].id == 76 and inv.slots[2].kind == 0, "decode inventory %s" % inv)
	var pc := Protocol.decode(golden["computer"].hex_decode())
	expect(pc.get("type") == Protocol.T_COMPUTER and pc.handle == 0xE001 and pc.owner == 3 and not pc.locked
		and pc.convs.size() == 3 and pc.convs[0].title == "#ogólny" and pc.convs[1].unread == 2
		and pc.convs[2].conv == Protocol.CONV_DM | 4, "decode computer %s" % pc)
	var ch := Protocol.decode(golden["chat"].hex_decode())
	expect(ch.get("type") == Protocol.T_CHAT and ch.conv == 17 and ch.messages.size() == 2
		and ch.messages[1].nick == "Kuba" and ch.messages[0].text == "Deploy w piątek?" and ch.messages[1].id == 6, "decode chat %s" % ch)
	var ml := Protocol.decode(golden["mail"].hex_decode())
	expect(ml.get("id") == 2 and ml.from == "Startup Sim — Rekrutacja" and ml.subject == "Zaproszenie na rozmowę"
		and ml.action == Protocol.PORTAL_JOIN_INTERVIEW and ml.arg == 1 and ml.body.begins_with("Cześć Ola"), "decode mail %s" % ml)
	var q := Protocol.decode(golden["question"].hex_decode())
	expect(q.get("attempt") == 3 and q.index == 1 and q.total == 3 and q.text == "Co oznacza kod HTTP 404?"
		and q.options.size() == 3 and q.options[1] == "Skończyła się kawa", "decode question %s" % q)
	var rr := Protocol.decode(golden["recruit_result"].hex_decode())
	expect(rr.get("attempt") == 3 and rr.passed == true and rr.score == 2 and rr.total == 3 and rr.department == 1, "decode recruit_result %s" % rr)
	var po := Protocol.decode(golden["pong"].hex_decode())
	expect(po.get("client_time") == 777000 and po.server_tick == 1234, "decode pong")
	var d := Protocol.decode(golden["disconnect"].hex_decode())
	expect(d.get("reason") == 1 and d.token == 0x01020304, "decode disconnect")
	var say := Protocol.decode(golden["say"].hex_decode())
	expect(say.get("type") == Protocol.T_SAY and say.id == 61440 and say.text == "Dzień dobry! Proszę za mną.", "decode say %s" % say)
	# Truncation must never decode.
	var snap: PackedByteArray = golden["snapshot"].hex_decode()
	for n in snap.size():
		expect(Protocol.decode(snap.slice(0, n)).is_empty(), "truncated snapshot len %d" % n)
	# Nick truncated on a character boundary (20 bytes -> 16).
	var c := Protocol.encode_connect(1, "ąąąąąąąąąą", {"gender": 0, "age": 20, "city": "X", "email": "a@b.c",
		"appearance": {"skin": 0, "hair_style": 0, "hair_color": 0, "shirt": 0, "pants": 0}})
	expect(c[8] == 16 and c.slice(9, 25).get_string_from_utf8() == "ąąąąąąąą", "nick truncation")


func _body_from(a: Array) -> Dictionary:
	return Movement.body(int(a[0]), Vector2i(int(a[1]), int(a[2])), int(a[3]), int(a[4]), int(a[5]))


func test_movement(path: String) -> void:
	var building = Building.new()
	building.load_path("res://maps/building.json")
	expect(building.error == "", "building loads: " + building.error)
	var data = load_json(path)
	expect(int(data["building_crc"]) == building.crc, "building crc parity %d vs %d" % [int(data["building_crc"]), building.crc])
	var case_i := 0
	var floor_changes := 0
	for c in data["cases"]:
		var b := _body_from(c["start"])
		var inputs: Array = c["inputs"]
		var states: Array = c["states"]
		var ok := true
		for i in inputs.size():
			var before: int = b.floor
			b = Movement.step(building, b, int(inputs[i]))
			if b.floor != before:
				floor_changes += 1
			var want := _body_from(states[i])
			if b != want:
				expect(false, "case %d step %d: got %s want %s" % [case_i, i, b, want])
				ok = false
				break
		expect(ok, "movement case %d" % case_i)
		case_i += 1
	expect(floor_changes >= 3, "vectors exercise stairs/elevator (%d floor changes)" % floor_changes)


func test_rejects_garbage() -> void:
	expect(Protocol.decode(PackedByteArray()).is_empty(), "empty")
	expect(Protocol.decode(PackedByteArray([0, 0, 1, 2])).is_empty(), "bad magic")
	expect(Protocol.decode(PackedByteArray([0x54, 0x53, 9, 2])).is_empty(), "bad version")
	expect(Protocol.decode(PackedByteArray([0x54, 0x53, 1, 200])).is_empty(), "unknown type")


func test_parse_address() -> void:
	var cases := {
		"127.0.0.1:7777": ["127.0.0.1", 7777],
		"127.0.0.1": ["127.0.0.1", 7777],
		"game.example.com:9000": ["game.example.com", 9000],
		"localhost": ["localhost", 7777],
		"[::1]:7000": ["::1", 7000],
		"[::1]": ["::1", 7777],
		"::1": ["::1", 7777],
		"2001:db8::5": ["2001:db8::5", 7777],
		"[2001:db8::5]:1234": ["2001:db8::5", 1234],
		"  10.0.0.2:80  ": ["10.0.0.2", 80],
		"": [],
		"host:": [],
		"host:abc": [],
		"host:70000": [],
		"[::1": [],
		"[::1]x": [],
		":7777": [],
	}
	for input in cases:
		var got := NetClient.parse_address(input)
		expect(got == cases[input], "parse_address(%s) = %s, want %s" % [input, got, cases[input]])
