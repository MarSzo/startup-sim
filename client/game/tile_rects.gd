## Tiles merged into few rectangles (runs along a row, stacked when the
## rows below repeat them) — so an overlay over a room is a handful of
## draw_rect calls instead of one per tile.
extends RefCounted


## `tiles`: Array of Vector2i; returns Array of Rect2 in pixels.
static func merge(tiles: Array, px: float) -> Array:
	var rows := {}
	for t in tiles:
		if not rows.has(t.y):
			rows[t.y] = []
		rows[t.y].append(t.x)
	var ys := rows.keys()
	ys.sort()
	var open := {}   # Vector2i(x0, x1) -> Rect2i (tiles), still growing down
	var out: Array[Rect2i] = []
	var last_y := -2
	for y in ys:
		var xs: Array = rows[y]
		xs.sort()
		var runs := []
		var start: int = xs[0]
		var prev: int = xs[0]
		for i in range(1, xs.size() + 1):
			if i < xs.size() and xs[i] == prev + 1:
				prev = xs[i]
				continue
			runs.append(Vector2i(start, prev))
			if i < xs.size():
				start = xs[i]
				prev = xs[i]
		var next := {}
		for run in runs:
			if y == last_y + 1 and open.has(run):
				var r: Rect2i = open[run]
				r.size.y += 1
				next[run] = r
				open.erase(run)
			else:
				next[run] = Rect2i(run.x, y, run.y - run.x + 1, 1)
		for run in open:
			out.append(open[run])
		open = next
		last_y = y
	for run in open:
		out.append(open[run])
	var px_rects := []
	for r in out:
		px_rects.append(Rect2(Vector2(r.position) * px, Vector2(r.size) * px))
	return px_rects
