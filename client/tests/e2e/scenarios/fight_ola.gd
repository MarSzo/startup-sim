## A fight (with fight_kuba): Ola takes a knife from the kitchen cupboard,
## meets Kuba in the chill room, stabs him until he's knocked out, the guard
## comes running after her; then a look at the R menu.
extends "res://tests/e2e/scenario.gd"

const Item = preload("res://game/item_art.gd")
const OUCH := ["Auć!", "Ała! Za co?!", "Ej, spokojnie!", "Oddam ci!", "Aaa! Nóż! Ratunku!"]


func run() -> void:
	if not await until(in_world, 30.0, "in the office"):
		return
	await until(func(): return holding(Item.LAPTOP), 5.0, "the laptop in hands")
	await item(Protocol.ITEM_DROP)
	if not await walk(1, 21, 8):  # in front of the kitchen cupboard
		return
	await press_e()
	var dialog = game().dialog
	if not await until(func(): return dialog.visible, 5.0, "the cupboard open"):
		return
	check(dialog._text.text.contains("noże kuchenne"), "knives inside: %s" % dialog._text.text)
	dialog._choose(1)  # Weź nóż
	if not await until(func(): return carrying(Item.KNIFE), 5.0, "a knife"):
		return
	log_step("took a knife")

	if not await walk(1, 34, 10):  # the chill room
		return
	if not await until(func(): return sees("Kuba"), 60.0, "Kuba in the chill room"):
		return
	await wait(1.0)
	var kuba := -1
	for id in game().nicks:
		if game().nicks[id] == "Kuba":
			kuba = id
	await item(Protocol.ITEM_TAKE_OUT, items().find(Item.KNIFE) - 1)
	if not await until(func(): return holding(Item.KNIFE), 5.0, "the knife in hands"):
		return
	var knocked_out := func(): return game().remotes.has(kuba) and game().remotes[kuba].status == Protocol.ACT_KNOCKED_OUT
	for n in 3:
		await item(Protocol.ITEM_USE)  # stab
		await wait(1.3)
	if not await until(knocked_out, 5.0, "Kuba knocked out"):
		return
	check(says.any(func(s): return s in OUCH), "Kuba felt it")
	log_step("Kuba knocked out")
	if not await hear("Hej! Bez bijatyk!", 5.0):
		return

	# The R menu: what could be done here (closed with the last option).
	game().net.send(Protocol.encode_action(game().net.token, Protocol.ACTION_MENU))
	if not await until(func(): return dialog.visible and dialog._title_text() == "Co zrobić?", 5.0, "the R menu"):
		return
	dialog._choose(dialog._opts.get_child_count() - 1)
	await until(func(): return not dialog.visible, 5.0, "the menu closed")
	# The guard catches up: a moment in place.
	await hear("Spokój! Bijatyki tu nie będzie", 40.0)
