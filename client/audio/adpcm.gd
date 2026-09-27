## IMA ADPCM (4 bits per sample) for voice chat: a frame is
## [predictor i16 LE, step index u8] + two samples per byte (low nibble
## first). 640 samples of 16 kHz mono (40 ms) -> 323 bytes.
extends RefCounted

const STEPS := [7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45,
	50, 55, 60, 66, 73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307,
	337, 371, 408, 449, 494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552,
	1707, 1878, 2066, 2272, 2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484,
	7132, 7845, 8630, 9493, 10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385,
	24623, 27086, 29794, 32767]
const INDEX_ADJ := [-1, -1, -1, -1, 2, 4, 6, 8]


## `samples`: -1..1 floats (an even count). Encoder state carries over from
## the previous frame in `state` ([predictor, index]) for smooth joins, but
## every frame also starts with it, so a lost frame doesn't break the next.
static func encode(samples: PackedFloat32Array, state: Array) -> PackedByteArray:
	var pred: int = state[0]
	var index: int = state[1]
	var out := PackedByteArray()
	out.resize(3 + samples.size() / 2)
	out[0] = pred & 0xff
	out[1] = (pred >> 8) & 0xff
	out[2] = index
	var byte := 0
	for i in samples.size():
		var s := clampi(int(samples[i] * 32767.0), -32768, 32767)
		var step: int = STEPS[index]
		var diff := s - pred
		var code := 0
		if diff < 0:
			code = 8
			diff = -diff
		var delta := step >> 3
		if diff >= step:
			code |= 4
			diff -= step
			delta += step
		step >>= 1
		if diff >= step:
			code |= 2
			diff -= step
			delta += step
		step >>= 1
		if diff >= step:
			code |= 1
			delta += step
		pred = clampi(pred - delta if code & 8 else pred + delta, -32768, 32767)
		index = clampi(index + INDEX_ADJ[code & 7], 0, 88)
		if i % 2 == 0:
			byte = code
		else:
			out[3 + i / 2] = byte | (code << 4)
	state[0] = pred
	state[1] = index
	return out


## Back to -1..1 floats.
static func decode(data: PackedByteArray) -> PackedFloat32Array:
	var out := PackedFloat32Array()
	if data.size() < 3:
		return out
	var pred := data[0] | (data[1] << 8)
	if pred >= 32768:
		pred -= 65536
	var index := clampi(data[2], 0, 88)
	out.resize((data.size() - 3) * 2)
	for i in out.size():
		var b := data[3 + i / 2]
		var code := b & 0x0f if i % 2 == 0 else b >> 4
		var step: int = STEPS[index]
		var delta := step >> 3
		if code & 4:
			delta += step
		if code & 2:
			delta += step >> 1
		if code & 1:
			delta += step >> 2
		pred = clampi(pred - delta if code & 8 else pred + delta, -32768, 32767)
		index = clampi(index + INDEX_ADJ[code & 7], 0, 88)
		out[i] = pred / 32768.0
	return out
