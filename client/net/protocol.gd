## Binary UDP protocol - mirror of server/src/protocol.rs (see docs/PROTOCOL.md).
## Little-endian. Header: magic u16 | version u8 | type u8.
extends RefCounted

const MAGIC := 0x5354
const VERSION := 38
const MAX_PACKET := 1152  # a game packet; sealed it grows to at most MAX_DATAGRAM
const MAX_DATAGRAM := 1200
const MAX_NICK_BYTES := 16
const MAX_SAY_BYTES := 240
const MAX_TEXT_BYTES := 240
const MAX_OPTIONS := 4
const MAX_INPUTS_PER_PACKET := 8

const T_CONNECT := 1
const T_WELCOME := 2
const T_REJECT := 3
const T_INPUT := 4
const T_SNAPSHOT := 5
const T_PLAYER_INFO := 6
const T_INFO_REQUEST := 7
const T_PING := 8
const T_PONG := 9
const T_DISCONNECT := 10
const T_SAY := 11
const T_JOB_OFFERS := 12
const T_APPLY := 13
const T_QUESTION := 14
const T_ANSWER := 15
const T_RECRUIT_RESULT := 16
const T_MAIL := 17
const T_PORTAL_ACTION := 18
const T_INVENTORY := 19
const T_ITEM_ACTION := 20
const T_COMPUTER := 21
const T_COMPUTER_ACTION := 22
const T_CHAT := 23
const T_STATS := 24
const T_DOORS := 25
const T_DOOR_ACTION := 26
const T_SHELF := 27
const T_SHOP_TAKE := 28
const T_CLOCK := 29
const T_COMMUTE_CHOICE := 30
const T_CALENDAR := 31
const T_CALENDAR_BOOK := 32
const T_DIALOG := 33
const T_DIALOG_ANSWER := 34
const T_LUNCH_MENU := 35
const T_LUNCH_ORDER := 36
const T_COMPANY_OFFERS := 37
const T_COMPANY_PEOPLE := 38
const T_COMPANY_ACTION := 39
const T_SMOKE := 40
const T_LIGHTS := 41
const T_FRIDGE := 42
const T_FRIDGE_ACTION := 43
const T_SKIP_WAIT := 44
const T_SOUND := 45
const T_TASK_ACTION := 46
const T_TASK_BOARD := 47
const T_TASK_DETAIL := 48
const T_MAIL_ACTION := 49
const T_WORK_MAIL := 50
const T_MAIL_STATE := 51
const T_VOICE := 52
const T_VOICE_FROM := 53
const T_DEPARTMENTS := 54
const MAX_VOICE_BYTES := 800
# TaskAction.action / MailAction.action (server/src/protocol/mod.rs)
const TA_SYNC := 0
const TA_CREATE := 1
const TA_MOVE := 2
const TA_ASSIGN := 3
const TA_PRIORITY := 4
const TA_COMMENT := 5
const TA_DELETE := 6
const TA_EDIT := 7
const MA_SYNC := 0
const MA_SEND := 1
const MA_TRASH := 2
const MA_RESTORE := 3
const MA_EMPTY_TRASH := 4
const TASK_TITLE_MAX := 80
const TASK_TEXT_MAX := 400
const TASK_COMMENT_MAX := 120
const MAIL_SUBJECT_MAX := 80
const MAIL_BODY_MAX := 400
## Sound.kind -> file in res://sounds (server/src/protocol/mod.rs `sound`).
const SOUND_FILES := {1: "coffee", 2: "till", 3: "gate_alarm", 4: "ding", 5: "lock", 6: "switch",
	7: "flush", 8: "tap", 9: "lighter", 10: "dishwasher", 11: "fridge", 12: "cupboard",
	13: "pickup", 14: "drop", 15: "eat", 16: "drink", 17: "whistle"}
# FridgeAction.action (server/src/kitchen.rs)
const FRIDGE_TAKE := 1
const FRIDGE_PUT := 2
const FRIDGE_WATER := 3
const FRIDGE_JUICE := 4
const FRIDGE_MILK := 5
# CompanyAction.action (server/src/company.rs)
const CO_FOUND := 1
const CO_RENAME := 2
const CO_SET_PLACES := 3
const CO_SET_DESCRIPTION := 4
const CO_HIRE := 5
const CO_REJECT := 6
const CO_FIRE := 7
const CO_ADD_POSITION := 8    # value = department, text = "title\nset\ndescription"
const CO_SET_TITLE := 9
const CO_SET_DEPARTMENT := 10
const CO_SET_QUESTIONS := 11  # text = question set id
const CO_REMOVE_POSITION := 12
const CO_MAX_PLACES := 5
const CO_MAX_POSITIONS := 10
# LunchMenu.state
const LUNCH_NONE := 0
const LUNCH_ORDERED := 1
const LUNCH_WAITING := 2
const LUNCH_CLOSED := 3
# Calendar slot states, meeting topics (server/src/board.rs).
const SLOT_FREE := 0
const SLOT_TAKEN := 1
const SLOT_MINE := 2
const SLOT_PAST := 3
const TOPICS := {1: "Prośba o podwyżkę", 2: "Pomysł na produkt", 3: "Skarga / problem", 4: "Luźna rozmowa"}
const TOPIC_WITH := {1: "z Prezesem", 2: "ze Wspólniczką", 3: "z Prezesem", 4: "z Prezesem"}
# Clock.place
const PLACE_BUILDING := 0
const PLACE_HOME := 1
const PLACE_COMMUTING := 2
const PLACE_PORTAL := 3
const NO_TIME := 0xFFFF

const PC_CLOSE := 1
const PC_LOCK := 2
const PC_UNLOCK := 3
const PC_TAKE := 4
const PC_SYNC := 5
const PC_SEND := 6
const MAX_CHAT_BYTES := 400
const MAX_CONVS := 40
# Messenger conversation ids (see server/src/computer.rs).
const CONV_GENERAL := 1
const CONV_DEPARTMENT_BASE := 16
const CONV_DM := 0x8000
# Computer entity flags.
const PC_FLAG_LOCKED := 1
const PC_FLAG_IN_USE := 2

const ITEM_TAKE_OUT := 1
const ITEM_PUT_AWAY := 2
const ITEM_DROP := 3
const ITEM_GIVE := 4
const ITEM_USE := 5

const PORTAL_NONE := 0
const PORTAL_JOIN_INTERVIEW := 1
const PORTAL_GO_TO_OFFICE := 2
const MAX_MAIL_BYTES := 600

const KIND_PLAYER := 0
const KIND_NPC := 1
const KIND_ITEM := 2
const KIND_COMPUTER := 3
const KIND_VEHICLE := 4
const KIND_TRAY := 5
const KIND_PUDDLE := 6

# What a character is doing: Snapshot.self_activity / entity activity.
const ACT_NONE := 0
const ACT_COMPUTER := 1
const ACT_BREWING := 2
const ACT_SOFA := 3
const ACT_TOILET := 4
const ACT_SMOKING := 5
const ACT_WASHING := 6
const ACT_RIDING := 7
const ACT_HELD := 8  # stopped by the guard / the police
# Entity flags bit 6: walks slowly (exhausted / needs the toilet).
const FLAG_SLOW := 0x40
# Entity flags bit 7: low hygiene (smell cloud).
const FLAG_SMELLY := 0x80
# Entity flags bit 3 (players only; NPC looks use bits 3-5): open umbrella.
const FLAG_UMBRELLA := 0x08
# Clock.weather
const WEATHER_SUNNY := 1
const WEATHER_CLOUDY := 2
const WEATHER_RAIN := 3
const WEATHER_STORM := 4
const WEATHER_FOG := 5
const WEATHER_NAMES := {1: "słonecznie", 2: "pochmurno", 3: "deszcz", 4: "burza", 5: "mgła"}
# Stats.flags bit 0: dirty hands.
const STATS_DIRTY_HANDS := 1
const STATS_UPSET := 2
# Doors.lift_target: the elevator isn't heading anywhere.
const NO_FLOOR := 255

const DISCONNECT_QUIT := 0
const DISCONNECT_TIMEOUT := 1
const DISCONNECT_KICKED := 2
const DISCONNECT_SHUTDOWN := 3
const DISCONNECT_SESSION_UNKNOWN := 4

const REJECT_REASONS := {1: "Serwer pełny", 2: "Ta wersja gry nie pasuje do serwera — pobierz najnowszą", 3: "Nieprawidłowe imię", 4: "Nieprawidłowe dane postaci",
	5: "Logowanie wygasło — zaloguj się ponownie", 6: "Ten serwer wymaga konta — zaloguj się", 7: "Ten nick jest zajęty — wybierz inny",
	8: "Ten e-mail ma już inna postać — wpisz inny"}
const REJECT_BAD_VERSION := 2
const REJECT_BAD_TICKET := 5
const REJECT_GUESTS_OFF := 6
const REJECT_NICK_TAKEN := 7
const REJECT_EMAIL_TAKEN := 8
const MAX_CITY_BYTES := 48
const MAX_EMAIL_BYTES := 64
const DISCONNECT_REASONS := {0: "Rozłączono", 1: "Przekroczono czas", 2: "Wyrzucono", 3: "Serwer wyłączony", 4: "Sesja wygasła"}


static func _writer(type: int) -> StreamPeerBuffer:
	var b := StreamPeerBuffer.new()
	b.big_endian = false
	b.put_u16(MAGIC)
	b.put_u8(VERSION)
	b.put_u8(type)
	return b


## UTF-8 bytes of `s`, cut to `max_bytes` on a character boundary.
static func utf8_truncated(s: String, max_bytes: int) -> PackedByteArray:
	var out := PackedByteArray()
	for ch in s:
		var cb := ch.to_utf8_buffer()
		if out.size() + cb.size() > max_bytes:
			break
		out.append_array(cb)
	return out


## profile: {gender, age, city, email, appearance: {skin, hair_style, hair_color, shirt, pants}}
## `ticket`: from logging in (HTTPS); "" = a guest.
static func encode_connect(nonce: int, nick: String, profile: Dictionary, ticket := "") -> PackedByteArray:
	var b := _writer(T_CONNECT)
	b.put_u32(nonce)
	var nb := utf8_truncated(nick, MAX_NICK_BYTES)
	b.put_u8(nb.size())
	b.put_data(nb)
	b.put_u8(profile.gender)
	b.put_u8(profile.age)
	_put_appearance(b, profile.appearance)
	_put_str16(b, profile.city, MAX_CITY_BYTES)
	_put_str16(b, profile.email, MAX_EMAIL_BYTES)
	_put_str16(b, ticket, 64)
	return b.data_array


static func _put_str16(b: StreamPeerBuffer, s: String, max_bytes: int) -> void:
	var sb := utf8_truncated(s, max_bytes)
	b.put_u16(sb.size())
	b.put_data(sb)


static func _put_appearance(b: StreamPeerBuffer, a: Dictionary) -> void:
	for k in ["skin", "hair_style", "hair_color", "shirt", "pants"]:
		b.put_u8(a[k])


## `inputs`: consecutive input bytes, oldest first; the last has seq `last_seq`.
static func encode_input(token: int, ack_tick: int, last_seq: int, inputs: PackedByteArray) -> PackedByteArray:
	var b := _writer(T_INPUT)
	b.put_u32(token)
	b.put_u32(ack_tick)
	b.put_u32(last_seq)
	var n := mini(inputs.size(), MAX_INPUTS_PER_PACKET)
	b.put_u8(n)
	b.put_data(inputs.slice(inputs.size() - n))
	return b.data_array


static func encode_info_request(token: int, ids: Array) -> PackedByteArray:
	var b := _writer(T_INFO_REQUEST)
	b.put_u32(token)
	var n := mini(ids.size(), 255)
	b.put_u8(n)
	for i in n:
		b.put_u16(ids[i])
	return b.data_array


static func encode_ping(token: int, client_time: int) -> PackedByteArray:
	var b := _writer(T_PING)
	b.put_u32(token)
	b.put_u32(client_time & 0xFFFFFFFF)
	return b.data_array


static func encode_apply(token: int, offer: int, motivation: String) -> PackedByteArray:
	var b := _writer(T_APPLY)
	b.put_u32(token)
	b.put_u8(offer)
	_put_str16(b, motivation, MAX_TEXT_BYTES)
	return b.data_array


static func encode_portal_action(token: int, action: int, arg: int) -> PackedByteArray:
	var b := _writer(T_PORTAL_ACTION)
	b.put_u32(token)
	b.put_u8(action)
	b.put_u8(arg)
	return b.data_array


static func encode_answer(token: int, attempt: int, index: int, choice: int) -> PackedByteArray:
	var b := _writer(T_ANSWER)
	b.put_u32(token)
	b.put_u8(attempt)
	b.put_u8(index)
	b.put_u8(choice)
	return b.data_array


static func encode_item_action(token: int, action: int, slot: int) -> PackedByteArray:
	var b := _writer(T_ITEM_ACTION)
	b.put_u32(token)
	b.put_u8(action)
	b.put_u8(slot)
	return b.data_array


static func encode_computer_action(token: int, action: int, conv: int, arg: int, text: String) -> PackedByteArray:
	var b := _writer(T_COMPUTER_ACTION)
	b.put_u32(token)
	b.put_u8(action)
	b.put_u16(conv)
	b.put_u32(arg)
	_put_str16(b, text, MAX_CHAT_BYTES)
	return b.data_array


static func encode_shop_take(token: int, shelf: int, kind: int) -> PackedByteArray:
	var b := _writer(T_SHOP_TAKE)
	b.put_u32(token)
	b.put_u8(shelf)
	b.put_u8(kind)
	return b.data_array


static func encode_commute_choice(token: int, mode: int) -> PackedByteArray:
	var b := _writer(T_COMMUTE_CHOICE)
	b.put_u32(token)
	b.put_u8(mode)
	return b.data_array


static func encode_calendar_book(token: int, start: int, topic: int) -> PackedByteArray:
	var b := _writer(T_CALENDAR_BOOK)
	b.put_u32(token)
	b.put_u16(start)
	b.put_u8(topic)
	return b.data_array


static func encode_company_action(token: int, action: int, target: int, value: int, text: String) -> PackedByteArray:
	var b := _writer(T_COMPANY_ACTION)
	b.put_u32(token)
	b.put_u8(action)
	b.put_u16(target)
	b.put_u8(value)
	_put_str16(b, text, MAX_TEXT_BYTES)
	return b.data_array


static func encode_task_action(token: int, nonce: int, action: int, task: int, arg: int, text: String) -> PackedByteArray:
	var b := _writer(T_TASK_ACTION)
	b.put_u32(token)
	b.put_u16(nonce)
	b.put_u8(action)
	b.put_u16(task)
	b.put_u8(arg)
	_put_str16(b, text, TASK_TEXT_MAX)
	return b.data_array


static func encode_mail_action(token: int, nonce: int, action: int, id: int, to: String, subject: String, body: String) -> PackedByteArray:
	var b := _writer(T_MAIL_ACTION)
	b.put_u32(token)
	b.put_u16(nonce)
	b.put_u8(action)
	b.put_u16(id)
	var tb := utf8_truncated(to, MAX_NICK_BYTES)
	b.put_u8(tb.size())
	b.put_data(tb)
	_put_str16(b, subject, MAIL_SUBJECT_MAX)
	_put_str16(b, body, MAIL_BODY_MAX)
	return b.data_array


static func encode_voice(token: int, seq: int, whisper: bool, data: PackedByteArray) -> PackedByteArray:
	var b := _writer(T_VOICE)
	b.put_u32(token)
	b.put_u16(seq & 0xffff)
	b.put_u8(1 if whisper else 0)
	var n := mini(data.size(), MAX_VOICE_BYTES)
	b.put_u16(n)
	b.put_data(data.slice(0, n))
	return b.data_array


static func encode_skip_wait(token: int) -> PackedByteArray:
	var b := _writer(T_SKIP_WAIT)
	b.put_u32(token)
	return b.data_array


static func encode_fridge_action(token: int, action: int, arg: int) -> PackedByteArray:
	var b := _writer(T_FRIDGE_ACTION)
	b.put_u32(token)
	b.put_u8(action)
	b.put_u8(arg)
	return b.data_array


static func encode_lunch_order(token: int, dish: int) -> PackedByteArray:
	var b := _writer(T_LUNCH_ORDER)
	b.put_u32(token)
	b.put_u8(dish)
	return b.data_array


static func encode_dialog_answer(token: int, id: int, choice: int) -> PackedByteArray:
	var b := _writer(T_DIALOG_ANSWER)
	b.put_u32(token)
	b.put_u8(id)
	b.put_u8(choice)
	return b.data_array


static func encode_door_action(token: int) -> PackedByteArray:
	var b := _writer(T_DOOR_ACTION)
	b.put_u32(token)
	return b.data_array


static func encode_disconnect(token: int, reason: int) -> PackedByteArray:
	var b := _writer(T_DISCONNECT)
	b.put_u32(token)
	b.put_u8(reason)
	return b.data_array


## Bounds-checked reader over a packet.
class Reader:
	var b := StreamPeerBuffer.new()
	var ok := true

	func _init(bytes: PackedByteArray) -> void:
		b.big_endian = false
		b.data_array = bytes

	func _has(n: int) -> bool:
		if not ok or b.get_available_bytes() < n:
			ok = false
			return false
		return true

	func u8() -> int:
		return b.get_u8() if _has(1) else 0

	func u16() -> int:
		return b.get_u16() if _has(2) else 0

	func u32() -> int:
		return b.get_u32() if _has(4) else 0

	func i32() -> int:
		return b.get_32() if _has(4) else 0

	func str8() -> String:
		var n := u8()
		if n > MAX_NICK_BYTES or not _has(n):
			ok = false
			return ""
		var res: Array = b.get_data(n)
		return (res[1] as PackedByteArray).get_string_from_utf8()

	func str16(max_bytes: int) -> String:
		var n := u16()
		if n > max_bytes or not _has(n):
			ok = false
			return ""
		var res: Array = b.get_data(n)
		return (res[1] as PackedByteArray).get_string_from_utf8()

	func bytes(n: int) -> PackedByteArray:
		if not _has(n):
			ok = false
			return PackedByteArray()
		var res: Array = b.get_data(n)
		return res[1]

	func at_end() -> bool:
		return b.get_available_bytes() == 0


## Decode a server->client packet. Returns {} for anything invalid.
static func decode(bytes: PackedByteArray) -> Dictionary:
	var r := Reader.new(bytes)
	if r.u16() != MAGIC or r.u8() != VERSION:
		return {}
	var t := r.u8()
	var p := {"type": t}
	match t:
		T_WELCOME:
			p.nonce = r.u32()
			p.player_id = r.u16()
			p.token = r.u32()
			p.tick_hz = r.u8()
			p.input_hz = r.u8()
			p.map_crc = r.u32()
			p.server_tick = r.u32()
		T_REJECT:
			p.reason = r.u8()
		T_SNAPSHOT:
			p.tick = r.u32()
			p.last_input_seq = r.u32()
			p.frag_idx = r.u8()
			p.frag_cnt = r.u8()
			p.self_x = r.i32()
			p.self_y = r.i32()
			p.floor = r.u8()
			p.room = r.u16()
			p.self_lock = r.u8()
			p.self_prev_input = r.u8()
			p.self_access = r.u8()
			p.self_slow = r.u8()
			p.self_activity = r.u8()
			var n := r.u8()
			var ents := []
			for i in n:
				ents.append({"id": r.u16(), "kind": r.u8(), "x": r.i32(), "y": r.i32(), "flags": r.u8(), "held": r.u8(), "activity": r.u8()})
			p.entities = ents
		T_PLAYER_INFO:
			var n := r.u8()
			var players := []
			for i in n:
				var e := {"id": r.u16(), "nick": r.str8(), "department": r.u8(), "gender": r.u8()}
				e.appearance = {"skin": r.u8(), "hair_style": r.u8(), "hair_color": r.u8(), "shirt": r.u8(), "pants": r.u8()}
				players.append(e)
			p.players = players
		T_PONG:
			p.client_time = r.u32()
			p.server_tick = r.u32()
		T_DISCONNECT:
			p.token = r.u32()
			p.reason = r.u8()
		T_SAY:
			p.id = r.u16()
			p.text = r.str16(MAX_SAY_BYTES)
		T_JOB_OFFERS:
			var n := r.u8()
			if n > 16:
				return {}
			var offers := []
			for i in n:
				offers.append({"id": r.u8(), "department": r.u8(), "applied": r.u8() != 0, "vacancies": r.u8(), "company": r.str16(MAX_TEXT_BYTES),
					"title": r.str16(MAX_TEXT_BYTES), "description": r.str16(MAX_TEXT_BYTES)})
			p.offers = offers
		T_QUESTION:
			p.attempt = r.u8()
			p.index = r.u8()
			p.total = r.u8()
			p.text = r.str16(MAX_TEXT_BYTES)
			var n := r.u8()
			if n > MAX_OPTIONS:
				return {}
			var options := []
			for i in n:
				options.append(r.str16(MAX_TEXT_BYTES))
			p.options = options
		T_RECRUIT_RESULT:
			p.attempt = r.u8()
			var passed := r.u8()
			if passed > 1:
				return {}
			p.passed = passed == 1
			p.score = r.u8()
			p.total = r.u8()
			p.department = r.u8()
		T_INVENTORY:
			var n := r.u8()
			if n > 8:
				return {}
			var slots := []
			for i in n:
				slots.append({"kind": r.u8(), "id": r.u32(), "label": r.str16(MAX_TEXT_BYTES)})
			p.slots = slots
		T_COMPUTER:
			p.handle = r.u16()
			p.owner = r.u16()
			p.locked = r.u8() != 0
			var n := r.u8()
			if n > MAX_CONVS:
				return {}
			var convs := []
			for i in n:
				convs.append({"conv": r.u16(), "unread": r.u8(), "title": r.str16(MAX_NICK_BYTES + 8)})
			p.convs = convs
		T_DOORS:
			p.floor = r.u8()
			var n := r.u8()
			var tiles := []
			for i in n:
				tiles.append(Vector2i(r.u8(), r.u8()))
			p.tiles = tiles
			# Every elevator, in the map's order (see Building.lift_ids).
			var lifts := []
			for i in r.u8():
				lifts.append({"floor": r.u8(), "target": r.u8(), "moving": r.u8() != 0})
			p.lifts = lifts
		T_STATS:
			p.hunger = r.u8()
			p.energy = r.u8()
			p.stress = r.u8()
			p.bladder = r.u8()
			p.hygiene = r.u8()
			p.stats_flags = r.u8()
			p.money = r.u32()
		T_CLOCK:
			p.day = r.u16()
			p.minute = r.u16()
			p.night = r.u8() != 0
			p.place = r.u8()
			p.arrive = r.u16()
			p.pay = r.u32()
			p.pay_minutes = r.u16()
			p.today_minutes = r.u16()
			p.mode = r.u8()
			p.depart = r.u16()
			p.money = r.u32()
			p.weather = r.u8()
			p.company = r.str16(64)
			p.founded = r.u8() != 0
			p.alarm = r.u8()
			p.skip = r.u8()
		T_TASK_BOARD:
			p.dept = r.u8()
			p.done = r.u16()
			p.part = r.u8()
			p.parts = r.u8()
			var nm := r.u8()
			if nm > 24:
				return {}
			var members := []
			for i in nm:
				members.append(r.str8())
			p.members = members
			var nt := r.u8()
			var tasks := []
			for i in nt:
				tasks.append({"id": r.u16(), "column": r.u8(), "priority": r.u8(), "comments": r.u8(),
					"title": r.str16(TASK_TITLE_MAX), "author": r.str8(), "assignee": r.str8()})
			p.tasks = tasks
		T_TASK_DETAIL:
			p.id = r.u16()
			p.desc = r.str16(TASK_TEXT_MAX)
			var nc := r.u8()
			if nc > 6:
				return {}
			var comments := []
			for i in nc:
				comments.append([r.str8(), r.str16(TASK_COMMENT_MAX)])
			p.comments = comments
		T_WORK_MAIL:
			p.id = r.u16()
			p.from = r.str8()
			p.to = r.str8()
			p.subject = r.str16(MAIL_SUBJECT_MAX)
			p.body = r.str16(MAIL_BODY_MAX)
			p.day = r.u16()
			p.minute = r.u16()
		T_MAIL_STATE:
			p.done = r.u16()
			for key in ["ids", "trashed"]:
				var n := r.u8()
				if n > 64:
					return {}
				var list := []
				for i in n:
					list.append(r.u16())
				p[key] = list
		T_DEPARTMENTS:
			var list := []
			for i in r.u8():
				list.append({"id": r.u8(), "short": r.str8(), "name": r.str8()})
			p.list = list
		T_VOICE_FROM:
			p.speaker = r.u16()
			p.seq = r.u16()
			p.whisper = r.u8() != 0
			var n := r.u16()
			if n > MAX_VOICE_BYTES:
				return {}
			p.data = r.bytes(n)
		T_SOUND:
			var n := r.u8()
			if n > 64:
				return {}
			var sounds := []
			for i in n:
				sounds.append([r.u8(), r.i32(), r.i32()])
			p.sounds = sounds
		T_FRIDGE:
			var n := r.u8()
			if n > 16:
				return {}
			var items := []
			for i in n:
				items.append({"kind": r.u8(), "label": r.str16(64)})
			p.items = items
			p.milk = r.u8()
			p.water = r.u8()
			p.juice = r.u8()
		T_LIGHTS:
			p.floor = r.u8()
			var n := r.u8()
			if n > 64:
				return {}
			var rooms := []
			for i in n:
				rooms.append(r.u16())
			p.rooms = rooms
		T_SMOKE:
			p.floor = r.u8()
			var n := r.u8()
			if n > 64:
				return {}
			var rooms := []
			for i in n:
				rooms.append([r.u16(), r.u8()])
			p.rooms = rooms
		T_COMPANY_OFFERS:
			p.name = r.str16(64)
			p.part = r.u8()
			p.parts = r.u8()
			var ns := r.u8()
			if ns > 16:
				return {}
			var sets := []
			for i in ns:
				sets.append({"id": r.str8(), "name": r.str16(64)})
			p.sets = sets
			var n := r.u8()
			if n > 16:
				return {}
			var offers := []
			for i in n:
				offers.append({"id": r.u8(), "places": r.u8(), "department": r.u8(), "set": r.str8(),
					"title": r.str16(64), "description": r.str16(MAX_TEXT_BYTES)})
			p.offers = offers
		T_COMPANY_PEOPLE:
			var n := r.u8()
			if n > 32:
				return {}
			var cands := []
			for i in n:
				cands.append({"id": r.u16(), "offer": r.u8(), "score": r.u8(), "total": r.u8(), "nick": r.str16(MAX_NICK_BYTES)})
			p.candidates = cands
			var m := r.u8()
			if m > 64:
				return {}
			var staff := []
			for i in m:
				staff.append({"id": r.u16(), "department": r.u8(), "day": r.u16(), "nick": r.str16(MAX_NICK_BYTES)})
			p.staff = staff
		T_CALENDAR:
			p.mine_start = r.u16()
			p.mine_topic = r.u8()
			var n := r.u8()
			if n > 64:
				return {}
			var slots := []
			for i in n:
				slots.append({"start": r.u16(), "state": r.u8()})
			p.slots = slots
		T_LUNCH_MENU:
			p.state = r.u8()
			p.dish = r.u8()
			p.arrives = r.u16()
			var n := r.u8()
			if n > 12:
				return {}
			var dishes := []
			for i in n:
				dishes.append({"kind": r.u8(), "price": r.u32(), "eta": r.u8(), "name": r.str16(64), "restaurant": r.str16(64)})
			p.dishes = dishes
		T_DIALOG:
			p.id = r.u8()
			p.npc = r.u16()
			p.text = r.str16(MAX_TEXT_BYTES)
			var n := r.u8()
			if n > MAX_OPTIONS:
				return {}
			var opts := []
			for i in n:
				opts.append(r.str16(MAX_TEXT_BYTES))
			p.options = opts
		T_SHELF:
			p.shelf = r.u8()
			p.title = r.str16(MAX_TEXT_BYTES)
			var n := r.u8()
			if n > 16:
				return {}
			var goods := []
			for i in n:
				goods.append({"kind": r.u8(), "price": r.u32(), "name": r.str16(MAX_TEXT_BYTES)})
			p.goods = goods
		T_CHAT:
			p.conv = r.u16()
			var n := r.u8()
			var msgs := []
			for i in n:
				msgs.append({"id": r.u32(), "from": r.u16(), "nick": r.str16(MAX_NICK_BYTES), "text": r.str16(MAX_CHAT_BYTES)})
			p.messages = msgs
		T_MAIL:
			p.id = r.u8()
			p.from = r.str16(MAX_TEXT_BYTES)
			p.subject = r.str16(MAX_TEXT_BYTES)
			p.body = r.str16(MAX_MAIL_BYTES)
			p.action = r.u8()
			p.arg = r.u8()
		_:
			return {}
	if not r.ok or not r.at_end():
		return {}
	return p
