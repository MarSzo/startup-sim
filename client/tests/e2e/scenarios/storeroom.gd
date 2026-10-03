## The reception's supplies (an employee, server at 11:58): a vitamin from
## the first-aid cabinet; the storeroom key - refused while the
## receptionist is at her desk, taken on her lunch break (12:00); a cola
## from the storeroom upstairs.
extends "res://tests/e2e/scenario.gd"

const Item = preload("res://game/item_art.gd")


func run() -> void:
	if not await until(in_world, 30.0, "in the office"):
		return
	await until(func(): return holding(Item.LAPTOP), 5.0, "the laptop in hands")
	await item(Protocol.ITEM_DROP)
	var dialog = game().dialog
	# The first-aid cabinet behind the reception desk.
	if not await walk(1, 39, 34, 60.0):
		return
	await press_e()
	if not await until(func(): return dialog.visible and dialog._title_text() == "Apteczka", 5.0, "the cabinet"):
		return
	dialog._choose(2)  # Witamina C
	if not await until(func(): return carrying(Item.VITAMIN), 5.0, "a vitamin"):
		return
	await item(Protocol.ITEM_TAKE_OUT, items().find(Item.VITAMIN) - 1)
	await item(Protocol.ITEM_USE)
	if not await hear("Witamina C."):
		return
	# The key: not while she's at the desk.
	if not await walk(1, 35, 34, 30.0):
		return
	await press_e()
	if not await hear("Klucz do magazynku? A po co"):
		return
	log_step("no key while she's here")
	# 12:00: off she goes for lunch - the key is free.
	if not await hear("Przerwa obiadowa", 60.0):
		return
	await wait(1.0)
	await press_e()
	if not await until(func(): return carrying(Item.STORE_KEY), 5.0, "the storeroom key"):
		return
	check(access() & MapData.ACCESS_KEY != 0, "the key opens the storeroom")
	log_step("took the key")
	# The storeroom: a cola off the shelf.
	if not await walk(1, 45, 40, 60.0):
		return
	await press_e()
	if not await until(func(): return dialog.visible and dialog._title_text() == "Magazynek", 5.0, "the storeroom shelves"):
		return
	dialog._choose(0)  # Coca-Cola
	if not await until(func(): return carrying(Item.COLA), 5.0, "a cola"):
		return
	log_step("a cola from the storeroom")
