## Another player: buffers server samples and renders them ~100 ms in the past,
## interpolating between the two samples around the render time.
extends "res://game/player_view.gd"

const MAX_SAMPLES := 32
const MAX_EXTRAPOLATE_TICKS := 2.0

var samples: Array = []  # [tick:int, pos:Vector2 (px), flags:int], ascending tick
var last_seen_tick := 0


func push_sample(tick: int, pos_px: Vector2, flags: int) -> void:
	last_seen_tick = tick
	if not samples.is_empty() and tick <= samples[-1][0]:
		return  # duplicate / out of order
	samples.append([tick, pos_px, flags])
	if samples.size() > MAX_SAMPLES:
		samples.pop_front()
	if samples.size() == 1:
		position = pos_px


## Returns true if the buffer ran dry (a moving player rendered past its
## newest sample) - used for the smoothness metric in the F3 overlay.
func update_render(render_tick: float) -> bool:
	if samples.is_empty():
		return false
	var n := samples.size()
	if render_tick <= samples[0][0]:
		position = samples[0][1]
		set_facing(samples[0][2] & 3)
		return false
	for i in range(n - 1, -1, -1):
		if samples[i][0] <= render_tick:
			if i < n - 1:
				var a: Array = samples[i]
				var b: Array = samples[i + 1]
				var t: float = (render_tick - a[0]) / float(b[0] - a[0])
				position = (a[1] as Vector2).lerp(b[1], t)
				set_facing(b[2] & 3)
				return false
			else:
				# Past the newest sample: short extrapolation, then hold.
				var last: Array = samples[n - 1]
				position = last[1]
				if n >= 2 and (last[2] & 4) != 0:
					var prev: Array = samples[n - 2]
					var vel: Vector2 = (last[1] - prev[1]) / float(last[0] - prev[0])
					position += vel * minf(render_tick - last[0], MAX_EXTRAPOLATE_TICKS)
				set_facing(last[2] & 3)
				return render_tick > last[0] and (last[2] & 4) != 0
	return false
