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
			"appearance": {"skin": 1, "hair_style": 4, "hair_color": 2, "shirt": 9, "pants": 3}}, "0a1b2c"),
		"input": Protocol.encode_input(0x01020304, 1200, 99, PackedByteArray([0, 1, 9, 6])),
		"info_request": Protocol.encode_info_request(0x01020304, [3, 4, 500]),
		"ping": Protocol.encode_ping(0x01020304, 777000),
		"apply": Protocol.encode_apply(0x01020304, 2, "Lubię kawę i wyzwania."),
		"portal_action": Protocol.encode_portal_action(0x01020304, Protocol.PORTAL_GO_TO_OFFICE, 0),
		"item_action": Protocol.encode_item_action(0x01020304, Protocol.ITEM_TAKE_OUT, 2),
		"door_action": Protocol.encode_door_action(0x01020304),
		"shop_take": Protocol.encode_shop_take(0x01020304, 1, 11),
		"commute_choice": Protocol.encode_commute_choice(0x01020304, 5),
		"calendar_book": Protocol.encode_calendar_book(0x01020304, 840, 2),
		"dialog_answer": Protocol.encode_dialog_answer(0x01020304, 3, 1),
		"lunch_order": Protocol.encode_lunch_order(0x01020304, 29),
		"company_action": Protocol.encode_company_action(0x01020304, Protocol.CO_SET_PLACES, 1, 2, ""),
		"fridge_action": Protocol.encode_fridge_action(0x01020304, Protocol.FRIDGE_PUT, 0),
		"skip_wait": Protocol.encode_skip_wait(0x01020304),
		"task_action": Protocol.encode_task_action(0x01020304, 7, Protocol.TA_CREATE, 0, 2, "Naprawić logowanie\nPo zmianie hasła."),
		"voice": Protocol.encode_voice(0x01020304, 9, true, PackedByteArray([0x10, 0x00, 0x05, 0x7f, 0x80])),
		"mail_action": Protocol.encode_mail_action(0x01020304, 4, Protocol.MA_SEND, 0, "Kuba", "Kawa?", "O 12 w kuchni."),
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
		and s.self_slow == 1 and s.self_activity == Protocol.ACT_SOFA
		and s.entities.size() == 2,
		"decode snapshot %s" % s)
	if s.has("entities") and s.entities.size() == 2:
		var e0: Dictionary = s.entities[0]
		var e1: Dictionary = s.entities[1]
		expect(e0.id == 3 and e0.kind == 0 and e0.x == 4096 and e0.y == 8192 and e0.flags == 5 and e0.held == 3 and e0.activity == Protocol.ACT_COMPUTER, "entity 0 %s" % e0)
		expect(e1.id == 65535 and e1.kind == 1 and e1.x == -1 and e1.y == 2000000 and e1.flags == Protocol.FLAG_SLOW, "entity 1 %s" % e1)
	var dr := Protocol.decode(golden["doors"].hex_decode())
	expect(dr.get("type") == Protocol.T_DOORS and dr.floor == 1 and dr.tiles == [Vector2i(46, 27), Vector2i(54, 31)]
		and dr.lift_floor == 0 and dr.lift_target == 1 and dr.lift_moving, "decode doors %s" % dr)
	var st := Protocol.decode(golden["stats"].hex_decode())
	expect(st.get("type") == Protocol.T_STATS and st.hunger == 35 and st.energy == 80 and st.stress == 12 and st.bladder == 64
		and st.hygiene == 22 and st.stats_flags == Protocol.STATS_DIRTY_HANDS and st.money == 18750, "decode stats %s" % st)
	var ck := Protocol.decode(golden["clock"].hex_decode())
	expect(ck.get("type") == Protocol.T_CLOCK and ck.day == 2 and ck.minute == 492 and not ck.night
		and ck.place == Protocol.PLACE_COMMUTING and ck.arrive == 545 and ck.pay == 23000 and ck.pay_minutes == 460
		and ck.today_minutes == 0 and ck.mode == 2 and ck.depart == 520 and ck.money == 18600
		and ck.weather == Protocol.WEATHER_RAIN and ck.company == "Pixel Pierogi sp. z o.o." and ck.founded and ck.alarm == 1 and ck.skip == 1, "decode clock %s" % ck)
	var fr := Protocol.decode(golden["fridge"].hex_decode())
	expect(fr.get("type") == Protocol.T_FRIDGE and fr.items.size() == 1 and fr.items[0].kind == 11
		and fr.items[0].label == "Kanapka z szynką (Ola)" and fr.milk == 7 and fr.water == 4 and fr.juice == 2, "decode fridge %s" % fr)
	var li := Protocol.decode(golden["lights"].hex_decode())
	expect(li.get("type") == Protocol.T_LIGHTS and li.floor == 1 and li.rooms == [5, 12], "decode lights %s" % li)
	var sm := Protocol.decode(golden["smoke"].hex_decode())
	expect(sm.get("type") == Protocol.T_SMOKE and sm.floor == 1 and sm.rooms == [[9, 40], [33, 200]], "decode smoke %s" % sm)
	var co := Protocol.decode(golden["company_offers"].hex_decode())
	expect(co.get("type") == Protocol.T_COMPANY_OFFERS and co.name.begins_with("Pixel") and co.offers.size() == 2
		and co.offers[0].places == 2 and co.offers[0].description == "Piszemy w Ruście.", "decode company offers %s" % co)
	var cp := Protocol.decode(golden["company_people"].hex_decode())
	expect(cp.get("type") == Protocol.T_COMPANY_PEOPLE and cp.candidates.size() == 1 and cp.candidates[0].nick == "Bob"
		and cp.staff.size() == 2 and cp.staff[1].day == 5, "decode company people %s" % cp)
	var cal := Protocol.decode(golden["calendar"].hex_decode())
	expect(cal.get("type") == Protocol.T_CALENDAR and cal.mine_start == 840 and cal.mine_topic == 1 and cal.slots.size() == 4
		and cal.slots[3].state == Protocol.SLOT_MINE and cal.slots[1].start == 630, "decode calendar %s" % cal)
	var lm := Protocol.decode(golden["lunch_menu"].hex_decode())
	expect(lm.get("type") == Protocol.T_LUNCH_MENU and lm.state == Protocol.LUNCH_ORDERED and lm.dish == 28 and lm.arrives == 760
		and lm.dishes.size() == 1 and lm.dishes[0].price == 2400 and lm.dishes[0].restaurant == "Pierogarnia u Zosi", "decode lunch menu %s" % lm)
	var dl := Protocol.decode(golden["dialog"].hex_decode())
	expect(dl.get("type") == Protocol.T_DIALOG and dl.id == 3 and dl.npc == 61444 and dl.options.size() == 2
		and dl.text.begins_with("Podwyżka"), "decode dialog %s" % dl)
	var sh := Protocol.decode(golden["shelf"].hex_decode())
	expect(sh.get("type") == Protocol.T_SHELF and sh.shelf == 1 and sh.title == "Kanapki" and sh.goods.size() == 2
		and sh.goods[1].name == "Kanapka z szynką" and sh.goods[1].price == 1400 and sh.goods[0].kind == 10, "decode shelf %s" % sh)
	var pi := Protocol.decode(golden["player_info"].hex_decode())
	expect(pi.get("players", []).size() == 2 and pi.players[0].nick == "Ala" and pi.players[0].department == 1
		and pi.players[0].gender == 0 and pi.players[0].appearance.hair_style == 1 and pi.players[0].appearance.hair_color == 3
		and pi.players[1].nick == "bot_07" and pi.players[1].id == 4 and pi.players[1].department == 0, "decode player_info %s" % pi)
	var jo := Protocol.decode(golden["job_offers"].hex_decode())
	expect(jo.get("offers", []).size() == 2 and jo.offers[1].title == "Dostawca/Dostawczyni" and jo.offers[1].department == 0
		and jo.offers[1].company == "Pizzeria u Stefana" and jo.offers[0].applied == true and jo.offers[1].applied == false
		and jo.offers[0].description == "Owocowe czwartki." and jo.offers[0].vacancies == 2, "decode job_offers %s" % jo)
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
	var tb := Protocol.decode(golden["task_board"].hex_decode())
	expect(tb.get("type") == Protocol.T_TASK_BOARD and tb.done == 7 and tb.members == ["Ola", "Kuba"] and tb.tasks.size() == 1
		and tb.tasks[0].title == "Naprawić logowanie" and tb.tasks[0].assignee == "Kuba" and tb.tasks[0].priority == 2, "decode task board %s" % tb)
	var td := Protocol.decode(golden["task_detail"].hex_decode())
	expect(td.get("id") == 3 and td.desc == "Po zmianie hasła." and td.comments == [["Kuba", "Zrobione"]], "decode task detail %s" % td)
	var wm := Protocol.decode(golden["work_mail"].hex_decode())
	expect(wm.get("from") == "HR" and wm.subject == "Witamy!" and wm.minute == 540 and wm.day == 2, "decode work mail %s" % wm)
	var ms := Protocol.decode(golden["mail_state"].hex_decode())
	expect(ms.get("done") == 4 and ms.ids == [1, 2, 5] and ms.trashed == [2], "decode mail state %s" % ms)
	var vf := Protocol.decode(golden["voice_from"].hex_decode())
	expect(vf.get("speaker") == 3 and vf.seq == 9 and not vf.whisper and vf.data == PackedByteArray([0x10, 0x00, 0x05, 0x7f, 0x80]), "decode voice_from %s" % vf)
	# ADPCM: a 440 Hz tone survives encoding (a rough SNR check).
	var Adpcm = load("res://audio/adpcm.gd")
	var adpcm_tone := PackedFloat32Array()
	for i in 640:
		adpcm_tone.append(0.5 * sin(TAU * 440.0 * i / 16000.0))
	var adpcm_state := [0, 0]
	var adpcm_enc: PackedByteArray = Adpcm.encode(adpcm_tone, adpcm_state)
	var adpcm_dec: PackedFloat32Array = Adpcm.decode(adpcm_enc)
	var adpcm_err := 0.0
	var adpcm_sig := 0.0
	for i in range(64, 640):  # after the step size adapts
		adpcm_err += pow(adpcm_dec[i] - adpcm_tone[i], 2)
		adpcm_sig += pow(adpcm_tone[i], 2)
	expect(adpcm_enc.size() == 323 and adpcm_dec.size() == 640, "adpcm sizes %d %d" % [adpcm_enc.size(), adpcm_dec.size()])
	expect(10.0 * log(adpcm_sig / maxf(adpcm_err, 1e-9)) / log(10.0) > 20.0, "adpcm SNR %.1f dB" % [10.0 * log(adpcm_sig / maxf(adpcm_err, 1e-9)) / log(10.0)])
	var snd := Protocol.decode(golden["sound"].hex_decode())
	expect(snd.get("type") == Protocol.T_SOUND and snd.sounds == [[1, 12288, -256], [17, 0, 65536]], "decode sound %s" % snd)
	# Truncation must never decode.
	var snap: PackedByteArray = golden["snapshot"].hex_decode()
	for n in snap.size():
		expect(Protocol.decode(snap.slice(0, n)).is_empty(), "truncated snapshot len %d" % n)
	# Nick truncated on a character boundary (20 bytes -> 16).
	var c := Protocol.encode_connect(1, "ąąąąąąąąąą", {"gender": 0, "age": 20, "city": "X", "email": "a@b.c",
		"appearance": {"skin": 0, "hair_style": 0, "hair_color": 0, "shirt": 0, "pants": 0}})
	expect(c[8] == 16 and c.slice(9, 25).get_string_from_utf8() == "ąąąąąąąą", "nick truncation")


func _body_from(a: Array) -> Dictionary:
	return Movement.body(int(a[0]), Vector2i(int(a[1]), int(a[2])), int(a[3]), int(a[4]), int(a[5]), int(a[6]) != 0)


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
	# Locked stall door (dynamic overlay): same result as stalls.rs in Rust.
	var m = building.get_floor(1)
	var walk_left := func() -> Dictionary:
		var bd := Movement.body(1, Movement.tile_center(48, 27), 0, Movement.LOCK_NONE, MapData.ACCESS_CARD)
		for i in 60:
			bd = Movement.step(building, bd, Movement.IN_LEFT)
		return bd
	expect(walk_left.call().pos.x < Movement.tile_center(46, 27).x, "open stall door: walks in")
	m.set_closed_tiles([Vector2i(46, 27)])
	expect(walk_left.call().pos.x == 47 * 256 + 5 * 16, "locked stall door stops at the door (%d)" % walk_left.call().pos.x)
	m.set_closed_tiles([])


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
