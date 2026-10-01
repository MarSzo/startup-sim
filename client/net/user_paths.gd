## Where the client keeps its own files (settings, remembered login, pinned
## certificates, the character draft). Normally right in user://; end-to-end
## scenarios (--scenario) get a folder of their own, so a test never touches
## the player's login or settings.
extends RefCounted

static var prefix := ""


static func at(file: String) -> String:
	return "user://" + prefix + file


## A separate, empty folder for this run (a scenario starts from scratch:
## no remembered login, no pinned certificates of an earlier test server).
static func use_folder(folder: String) -> void:
	prefix = folder.trim_suffix("/") + "/"
	var path := "user://" + prefix
	DirAccess.make_dir_recursive_absolute(path)
	for f in DirAccess.get_files_at(path):
		DirAccess.remove_absolute(path + f)
