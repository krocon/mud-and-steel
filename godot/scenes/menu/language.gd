extends RefCounted
## Interface language (prompt 043). Static, like `settings.gd`.
##
## UI texts are keys (`SETTINGS_TITLE`) looked up in Godot translations built
## from `res://locale/*.csv` (columns `keys,en,de`; Godot imports each file
## into `<file>.<lang>.translation`, see `docs/ARCHITECTURE.md`). A `Control`
## whose `text` is a key shows the translation by itself and follows a
## language change; texts built with `%` use `tr("KEY") % [...]` and are
## rebuilt by their screen.
##
## Unit and object names come from the catalog (`name_de`/`name_en`) through
## `SimHost.set_language`; mission texts from `<mission>.<lang>.po` next to
## the mission file (`missions.gd`).
##
## `install()` registers every translation in `res://locale/` (no list in
## `project.godot`, so a new area file only needs `just import`).

## Languages offered in the settings, by locale code, with their own name
## (never translated, so a player always finds their language).
const LANGUAGES: Dictionary[String, String] = {"en": "English", "de": "Deutsch"}
const FALLBACK := "en"
const LOCALE_DIR := "res://locale"

static var _installed := false
static var current := FALLBACK


## The language `setting` stands for: `auto` follows the system language
## when it is offered, otherwise English.
## `--language=<code>` on the command line wins; self-tests (any
## `--…self-test` argument) run in English unless they set a language.
static func resolve(setting: String) -> String:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--language=") and arg.trim_prefix("--language=") in LANGUAGES:
			return arg.trim_prefix("--language=")
	if setting in LANGUAGES:
		return setting
	for arg in OS.get_cmdline_user_args():
		if arg.ends_with("self-test"):
			return FALLBACK
	var system := OS.get_locale_language()
	return system if system in LANGUAGES else FALLBACK


## Registers the translations once.
static func install() -> void:
	if _installed:
		return
	_installed = true
	for file in DirAccess.get_files_at(LOCALE_DIR):
		var name := file.trim_suffix(".remap")
		if name.ends_with(".translation"):
			var translation := load(LOCALE_DIR.path_join(name)) as Translation
			if translation != null:
				TranslationServer.add_translation(translation)


## Switches the interface, unit names and mission texts to `setting`.
static func apply(setting: String) -> void:
	install()
	current = resolve(setting)
	if TranslationServer.get_locale() != current:
		TranslationServer.set_locale(current)
	SimHost.set_language(current)


## True if `text` looks like a translation key (upper case, digits and at
## least one underscore).
static func is_key(text: String) -> bool:
	if text.length() < 3 or not text.contains("_"):
		return false
	for c in text:
		if not (c == "_" or (c >= "A" and c <= "Z") or (c >= "0" and c <= "9")):
			return false
	return text[0] >= "A" and text[0] <= "Z"


## The translation of `key` in the current language, for static functions
## (which have no `tr`).
static func t(key: String) -> String:
	return String(TranslationServer.translate(key))


## True if `key` has a translation in the current language.
static func has(key: String) -> bool:
	return TranslationServer.translate(key) != StringName(key)


## The translation of `prefix + code` (code upper-cased), or `fallback` when
## there is none: for names from data files (`RANK_VETERAN`) and codes from
## the simulation (`BLOCKED_NO_FUEL`).
static func code_text(prefix: String, code: String, fallback := "") -> String:
	var key := prefix + code.to_upper()
	var text := TranslationServer.translate(key)
	if text == StringName(key):
		return fallback if not fallback.is_empty() else code
	return String(text)
