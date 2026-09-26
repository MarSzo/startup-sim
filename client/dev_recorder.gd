## Dev: records the window as a numbered JPG sequence at a steady real-time
## rate (for trailers; `ffmpeg -framerate 30 -i dir/%05d.jpg`). Late frames
## are filled with the last image, so the sequence keeps real time with the
## server. Files are written on worker threads.
##   --record=/dir [--record-start=2] [--record-length=6] [--record-fps=30]
## Quits the game when done.
extends Node

var dir := ""
var start := 0.0
var length := 6.0
var fps := 30.0
var _t := 0.0
var _next := 0
var _tasks: Array[int] = []
var _done := false


func _ready() -> void:
	process_mode = Node.PROCESS_MODE_ALWAYS
	DirAccess.make_dir_recursive_absolute(dir)


func _process(delta: float) -> void:
	if _done:
		return
	_t += delta
	var t := _t - start
	if t < 0.0:
		return
	var want := int(t * fps)
	var total := int(length * fps)
	if want >= _next:
		var img := get_viewport().get_texture().get_image()
		while _next <= mini(want, total - 1):
			var path := "%s/%05d.jpg" % [dir, _next]
			_tasks.append(WorkerThreadPool.add_task(func(): img.save_jpg(path, 0.92)))
			_next += 1
	if _next >= total:
		_done = true
		for id in _tasks:
			WorkerThreadPool.wait_for_task_completion(id)
		print("recorded %d frames to %s" % [_next, dir])
		get_tree().quit()
