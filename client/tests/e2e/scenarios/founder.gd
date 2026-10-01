## The founder (with founder_hire): founds the company from the job portal
## (--found), puts the laptop on the board table, adds a position in the
## Mobile department in the company panel and hires the first candidate.
extends "res://tests/e2e/scenario.gd"

const Item = preload("res://game/item_art.gd")
const MOBILE := 4


func run() -> void:
	if not await until(in_world, 60.0, "the company founded, in the board room"):
		return
	await until(func(): return holding(Item.LAPTOP) and carrying(Item.EMPLOYEE_CARD), 5.0, "the card and the laptop")
	if not check(room_name() == "Zarząd", "in the board room (in %s)" % room_name()):
		return
	log_step("founded")
	if not await sit_at_desk():
		return
	await pc("company")
	var screen = game().screen
	if not await until(func(): return not screen.company_offers.is_empty(), 10.0, "the company panel"):
		return
	await pc("company:%d:0:%d:Programista/ka mobile\nprogramming\nAplikacje na telefony." % [Protocol.CO_ADD_POSITION, MOBILE])
	var mobile_open := func():
		for o in screen.company_offers.get("offers", []):
			if o.department == MOBILE and o.title == "Programista/ka mobile":
				return o.places
		return -1
	if not await until(func(): return mobile_open.call() == 1, 10.0, "the Mobile position on the panel"):
		return
	log_step("position added in Mobile")

	if not await until(func(): return not screen.company_people.get("candidates", []).is_empty(), 150.0, "a candidate"):
		return
	await pc("company:hire")
	if not await until(func(): return mobile_open.call() == 0, 10.0, "the place taken"):
		return
	log_step("hired")
	await wait(4.0)  # the applicant gets the invitation before we leave
