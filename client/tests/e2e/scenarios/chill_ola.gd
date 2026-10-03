## The chill room (with chill_kuba): Ola takes the remote and puts the
## match on, then takes the boombox and plays disco polo - Kuba sees and
## hears the same.
extends "res://tests/e2e/scenario.gd"

const Item = preload("res://game/item_art.gd")


func run() -> void:
	if not await until(in_world, 30.0, "in the office"):
		return
	await until(func(): return holding(Item.LAPTOP), 5.0, "the laptop in hands")
	await item(Protocol.ITEM_DROP)
	if not await walk(1, 34, 12, 90.0):  # the remote lies here
		return
	await press_e()
	if not await until(func(): return carrying(Item.REMOTE), 5.0, "the remote"):
		return
	await item(Protocol.ITEM_TAKE_OUT, items().find(Item.REMOTE) - 1)
	await item(Protocol.ITEM_USE)
	var dialog = game().dialog
	if not await until(func(): return dialog.visible and dialog._title_text() == "Telewizor", 5.0, "the channels"):
		return
	dialog._choose(3)  # Mecz
	if not await hear("Przełączam na: Mecz"):
		return
	await until(func(): return not game().tvs.is_empty() and game().tvs.values()[0][0].channel == 4, 5.0, "the match on")
	await item(Protocol.ITEM_PUT_AWAY)
	if not await walk(1, 41, 12, 30.0):  # the boombox
		return
	await press_e()
	if not await until(func(): return holding(Item.BOOMBOX), 5.0, "the boombox"):
		return
	await item(Protocol.ITEM_USE)
	if not await until(func(): return dialog.visible and dialog._title_text() == "Boombox", 5.0, "the tracks"):
		return
	dialog._choose(0)  # disco polo
	await until(func(): return game().boombox_music.get("track", 0) == 1, 5.0, "music on")
	log_step("TV and music on")
	await wait(6.0)  # Kuba checks
