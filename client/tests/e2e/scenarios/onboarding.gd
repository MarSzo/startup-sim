## A new player (server with accounts): register, create the character, the
## job portal (applies and answers by itself: --auto-recruit), off to the
## office; the porter, the reception and HR - the card and the laptop; the
## laptop on a desk of the department.
extends "res://tests/e2e/scenario.gd"

const Item = preload("res://game/item_art.gd")

## The variant "resign": turn the contract down at HR (see resign.gd).
var resign := false


func run() -> void:
	# Registration, the character, the portal and the interview go by
	# themselves (--register --autocreate --auto-recruit); retries included.
	if not await until(in_world, 180.0, "hired and in front of the office"):
		return
	await until(func(): return tile_now().y >= 58, 30.0, "out on the sidewalk")
	log_step("arrived, access %d" % access())
	check(access() == 0, "no pass yet")

	if not await walk(0, 34, 49):  # across the porter's desk
		return
	await press_e()
	if not await hear("Dzień dobry! Pierwszy dzień?"):
		return
	await until(func(): return access() & MapData.ACCESS_GUEST != 0, 5.0, "a guest pass")
	log_step("guest pass from the porter")

	# Follow him up to the reception (he says when we're there).
	if not await walk(1, 36, 36, 90.0):
		return
	if not await hear("To recepcja", 30.0):
		return
	await press_e()
	if not await hear("Witamy! Zaprowadzę do HR"):
		return
	if not await walk(1, 47, 14, 60.0):  # HR
		return
	if not await hear("To dział HR", 30.0):
		return
	await press_e()
	# The contract: a little less than agreed (of course). Signed anyway.
	var dialog = game().dialog
	if not await until(func(): return dialog.visible and dialog._title_text().begins_with("Umowa"), 5.0, "the contract"):
		return
	check(dialog._text.text.contains("brutto miesięcznie") and dialog._text.text.contains("korekta"), "the pay on paper: %s" % dialog._text.text)
	if resign:
		await turn_it_down(dialog)
		return
	dialog._choose(0)  # Podpisuję
	if not await until(func(): return carrying(Item.EMPLOYEE_CARD) and holding(Item.LAPTOP), 10.0, "the card and the laptop from HR"):
		return
	await until(func(): return access() & MapData.ACCESS_CARD != 0, 5.0, "the card opening the gates")
	check(not carrying(Item.GUEST_PASS), "the guest pass taken back")
	log_step("employed, department %d" % game().department)

	var desk := desk_of(game().department)
	if not check(not desk.is_empty(), "a desk of department %d" % game().department):
		return
	if not await walk(desk[0], desk[1].x, desk[1].y, 60.0):
		return
	await press_e()
	await hear("Laptop na biurku")


## "Rezygnuję": HR walks us down to the porter's desk, Pani Wiesia takes the
## pass, and it's the job portal again (with a mail about it).
func turn_it_down(dialog) -> void:
	dialog._choose(1)
	if not await hear("Szkoda. Odprowadzę na portiernię"):
		return
	if not await walk(0, 34, 49, 90.0):  # with HR, to the porter's desk
		return
	if not await hear("Przepustkę poproszę", 40.0):
		return
	await until(func(): return access() == 0, 5.0, "the pass taken back")
	log_step("the pass given back")
	var portal = main.portal
	await until(func():
		for m in portal.mails.values():
			if str(m.subject) == "Rezygnacja z umowy":
				return true
		return false, 20.0, "back on the job portal")
