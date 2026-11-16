extends Node
## Keeps a local crash report when the game ended unexpectedly (prompt 044).
##
## Godot writes its log to `user://logs/godot.log` (project setting
## `debug/file_logging`), including the backtrace of a crash; on the next
## start it renames that file to `godot<date-time>.log`. This autoload writes
## `user://session.lock` at start and removes it on a clean exit. A lock found
## at start means the last session crashed (or was killed): its log is copied
## to `user://crash-reports/crash-<date-time>.log` with a short header. The
## main menu then says where the report is. Nothing is ever sent anywhere.
##
## Only active in an exported game (in the repository the log is in the
## terminal, and the self-tests kill processes on purpose); `-- --crash-guard`
## turns it on for testing.

const LOCK := "user://session.lock"
const LOG_DIR := "user://logs"
const REPORT_DIR := "user://crash-reports"
## Older reports beyond this count are deleted.
const KEEP_REPORTS := 10

## Absolute path of the report written at this start, or "".
static var last_report := ""

var _active := false


func _enter_tree() -> void:
	_active = GamePaths.is_exported() or "--crash-guard" in OS.get_cmdline_user_args()
	if not _active:
		return
	if FileAccess.file_exists(LOCK):
		last_report = _write_report(FileAccess.get_file_as_string(LOCK))
		if not last_report.is_empty():
			print("CrashGuard: the last session ended unexpectedly; report: %s" % last_report)
	var lock := FileAccess.open(LOCK, FileAccess.WRITE)
	if lock != null:
		lock.store_string("version %s, started %s, %s\n" % [
			ProjectSettings.get_setting("application/config/version", "?"),
			Time.get_datetime_string_from_system(true), OS.get_name(),
		])


func _exit_tree() -> void:
	if _active:
		DirAccess.remove_absolute(ProjectSettings.globalize_path(LOCK))


## Copies the newest rotated log (the crashed session's) into a report.
func _write_report(session: String) -> String:
	var logs := DirAccess.get_files_at(LOG_DIR)
	var previous := ""
	for file in logs:
		if file.begins_with("godot") and file.ends_with(".log") and file != "godot.log" and file > previous:
			previous = file
	DirAccess.make_dir_recursive_absolute(REPORT_DIR)
	var stamp := Time.get_datetime_string_from_system(true).replace(":", "-")
	var path := REPORT_DIR.path_join("crash-%s.log" % stamp)
	var out := FileAccess.open(path, FileAccess.WRITE)
	if out == null:
		return ""
	out.store_string("Mud and Steel crash report (kept locally, never sent)\n")  # l10n-ignore (log file)
	out.store_string("Session: %s" % session)
	out.store_string("Report written by version %s on %s\n\n" % [  # l10n-ignore (log file)
		ProjectSettings.get_setting("application/config/version", "?"), OS.get_name(),
	])
	if previous.is_empty():
		out.store_string("(no log of the last session found)\n")
	else:
		out.store_string("--- %s ---\n" % previous)
		out.store_string(FileAccess.get_file_as_string(LOG_DIR.path_join(previous)))
	out.close()
	_prune()
	return ProjectSettings.globalize_path(path)


func _prune() -> void:
	var reports := Array(DirAccess.get_files_at(REPORT_DIR))
	reports.sort()
	while reports.size() > KEEP_REPORTS:
		DirAccess.remove_absolute(ProjectSettings.globalize_path(REPORT_DIR.path_join(reports.pop_front())))
