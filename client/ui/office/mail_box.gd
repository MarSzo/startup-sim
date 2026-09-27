## The work inbox of the computer's owner, as the client knows it: mails by
## id, which are in the trash, which were read here. Keeps itself in sync
## (MailAction SYNC with the newest id) and resends actions until the
## server's MailState says they were applied.
extends RefCounted

const Protocol = preload("res://net/protocol.gd")

## MailAction to send.
signal send(nonce: int, action: int, id: int, to: String, subject: String, body: String)
## Something changed (redraw).
signal changed
## New mail arrived after the first load.
signal arrived(mail: Dictionary)

const SYNC_MSEC := 2000
const RESEND_MSEC := 800

var owner := -1
var mails := {}        # id -> mail dict
var trashed := {}      # id -> true
var read := {}         # id -> true
var _loaded := false   # the first MailState came (no "new mail" for the old ones)
var _nonce := randi_range(1, 30000)
var _queue: Array = [] # [nonce, action, id, to, subject, body, msec]
var _last_sync := -SYNC_MSEC


func reset(p_owner: int) -> void:
	owner = p_owner
	mails.clear()
	trashed.clear()
	read.clear()
	_queue.clear()
	_loaded = false
	_last_sync = -SYNC_MSEC
	changed.emit()


func newest() -> int:
	var m := 0
	for id in mails:
		m = maxi(m, id)
	return m


func inbox() -> Array:
	return _sorted(func(id): return not trashed.has(id))


func trash() -> Array:
	return _sorted(func(id): return trashed.has(id))


func unread() -> int:
	var n := 0
	for id in mails:
		if not trashed.has(id) and not read.has(id):
			n += 1
	return n


func _sorted(keep: Callable) -> Array:
	var out := []
	for id in mails:
		if keep.call(id):
			out.append(mails[id])
	out.sort_custom(func(a, b): return a.id > b.id)
	return out


func on_mail(p: Dictionary) -> void:
	if mails.has(p.id):
		return
	mails[p.id] = p
	if _loaded:
		arrived.emit(p)
	else:
		read[p.id] = true  # what was there before we sat down counts as seen
	changed.emit()


func on_state(p: Dictionary) -> void:
	var exists := {}
	for id in p.ids:
		exists[id] = true
	for id in mails.keys():
		if not exists.has(id):
			mails.erase(id)  # the trash was emptied
	trashed.clear()
	for id in p.trashed:
		trashed[id] = true
	while not _queue.is_empty() and ((p.done - _queue[0][0]) & 0xffff) < 0x8000:
		_queue.pop_front()
	for q in _queue:  # not applied yet: still show them
		_apply(q[1], q[2])
	# More to fetch: ask right away.
	var missing := false
	for id in p.ids:
		if not mails.has(id) and id > newest():
			missing = true
	if not missing:
		_loaded = true
	else:
		_last_sync = -SYNC_MSEC
	changed.emit()


## Every frame while at the computer.
func tick() -> void:
	var now := Time.get_ticks_msec()
	if not _queue.is_empty() and now - _queue[0][6] >= RESEND_MSEC:
		var q: Array = _queue[0]
		q[6] = now
		send.emit(q[0], q[1], q[2], q[3], q[4], q[5])
	elif now - _last_sync >= SYNC_MSEC:
		_last_sync = now
		send.emit(0, Protocol.MA_SYNC, newest(), "", "", "")


func act(action: int, id: int, to := "", subject := "", body := "") -> void:
	_nonce = _nonce % 65535 + 1
	var q := [_nonce, action, id, to, subject, body, Time.get_ticks_msec()]
	_queue.append(q)
	if _queue.size() == 1:
		send.emit(q[0], q[1], q[2], q[3], q[4], q[5])
	_apply(action, id)
	changed.emit()


## Show an action at once; the server's state confirms it.
func _apply(action: int, id: int) -> void:
	match action:
		Protocol.MA_TRASH:
			trashed[id] = true
		Protocol.MA_RESTORE:
			trashed.erase(id)
		Protocol.MA_EMPTY_TRASH:
			for t in trashed.keys():
				mails.erase(t)
			trashed.clear()
