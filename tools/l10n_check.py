#!/usr/bin/env python3
"""Localization check (prompt 043): run by `just l10n-check` and `just selftest`.

1. Every translation table `godot/locale/*.csv` has the header `keys,en,de`,
   unique keys in KEY form, a text in every language, and the same
   placeholders (`%s`, `%d`, `%.1f`, `{name}`) in every language.
2. Every key literal in the game scripts exists in a table.
3. No hard-coded interface text in the game scripts: a string literal that
   reads like prose (words, a capital letter followed by lower case) is an
   error unless the line is logging, a node or file name, or carries the
   marker `# l10n-ignore` (log texts, file formats, proper names).

Scripts of developer tools stay English and are not checked: `scenes/tools/`,
`scenes/debug/`, and the self-tests (`*self_test*.gd`). See
docs/ARCHITECTURE.md, "Translations".

Exit code 0 when everything is fine, 1 otherwise. `--unused` also lists keys
no script names literally (many are built at run time, so only a hint).
"""

import csv
import os
import re
import sys

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
LOCALE = os.path.join(ROOT, "godot", "locale")
SCENES = os.path.join(ROOT, "godot", "scenes")
LANGUAGES = ["en", "de"]
SKIP_DIRS = {"tools", "debug"}
MARKER = "l10n-ignore"

KEY = re.compile(r"^[A-Z][A-Z0-9]*(_[A-Z0-9]+)+$")
KEY_PREFIX = re.compile(r"^[A-Z][A-Z0-9]*(_[A-Z0-9]+)*_$")
PLACEHOLDER = re.compile(r"%(?:[-+ 0#]*\d*(?:\.\d+)?[sdifxXc])|\{[a-z_0-9]+\}")
STRING = re.compile(r'(&|\^)?"((?:[^"\\]|\\.)*)"')
# Lines whose strings never reach the player.
LOGGING = re.compile(
    r"\b(print|printerr|print_rich|push_error|push_warning|assert|prints|print_debug|"
    r"_check|_fail|_note|_log|godot_error)\s*\("
)
NODE_NAMES = re.compile(
    r"(\.name\s*=|\bname\s*=\s*\"|get_node(_or_null)?\(|has_node\(|find_child\(|"
    r"\bnode_path\b|NodePath\(|has_meta\(|get_meta\(|set_meta\(|"
    r"is_action|InputMap|add_theme_|get_theme_|has_theme_|\bConfigFile\b|"
    r"get_value\(|set_value\(|\bcall\(|\bconnect\(|has_method\(|has_signal\()"
)
# Lines that set interface text.
UI_LINE = re.compile(
    r"(\.text\s*\+?=|\btext\s*=|tooltip_text|placeholder_text|dialog_text|\.title\s*=|"
    r"add_item\(|add_check_item\(|add_radio_check_item\(|set_item_text\(|set_tab_title\(|"
    r"\btitle\(|\bhint\(|_row\(|_label\(|_button\(|_heading\(|append_text\(|add_text\(|"
    r"draw_string\(|\btr\()"
)
PROSE = re.compile(r"[A-Za-z]*[a-z]{2,}")


def load_tables():
    errors = []
    keys = {}
    for name in sorted(os.listdir(LOCALE)):
        if not name.endswith(".csv"):
            continue
        path = os.path.join(LOCALE, name)
        with open(path, newline="", encoding="utf-8") as handle:
            rows = list(csv.reader(handle))
        if not rows or rows[0][: len(LANGUAGES) + 1] != ["keys"] + LANGUAGES:
            errors.append(f"locale/{name}: header must start with keys,{','.join(LANGUAGES)}")
            continue
        for number, row in enumerate(rows[1:], start=2):
            where = f"locale/{name}:{number}"
            if not row or not row[0]:
                continue
            key = row[0]
            if not KEY.match(key):
                errors.append(f"{where}: '{key}' is no key (UPPER_CASE with an underscore)")
            if key in keys:
                errors.append(f"{where}: key {key} also in {keys[key]}")
            keys[key] = where
            texts = (row + [""] * len(LANGUAGES))[1 : len(LANGUAGES) + 1]
            for lang, text in zip(LANGUAGES, texts):
                if not text.strip():
                    errors.append(f"{where}: {key} has no '{lang}' text")
            wanted = sorted(PLACEHOLDER.findall(texts[0]))
            for lang, text in zip(LANGUAGES[1:], texts[1:]):
                got = sorted(PLACEHOLDER.findall(text))
                if got != wanted and text.strip():
                    errors.append(f"{where}: {key} placeholders {got} in '{lang}', {wanted} in 'en'")
    return keys, errors


def scripts():
    for folder, dirs, files in os.walk(SCENES):
        rel = os.path.relpath(folder, SCENES)
        if rel.split(os.sep)[0] in SKIP_DIRS:
            dirs[:] = []
            continue
        for name in sorted(files):
            if name.endswith(".gd") and "self_test" not in name:
                yield os.path.join(folder, name)


def code_part(line):
    """The line without its comment (a `#` outside a string)."""
    quote = False
    escaped = False
    for i, c in enumerate(line):
        if escaped:
            escaped = False
        elif c == "\\":
            escaped = True
        elif c == '"':
            quote = not quote
        elif c == "#" and not quote:
            return line[:i]
    return line


def is_prose(text, ui_line):
    """True for interface text: words with a capital start ("Save game",
    "%d units left"), or any word on a line that sets interface text."""
    if not text.strip() or KEY.match(text) or KEY_PREFIX.match(text):
        return False
    if text.startswith(("res://", "user://", "uid://", "--")) or re.search(r"\.[a-z0-9]{2,5}$", text):
        return False
    # File paths ("../Resources/data").
    if " " not in text and "/" in text:
        return False
    # snake_case ids, codes, single lower-case words, node paths.
    if re.fullmatch(r"[a-z0-9_./:@*\-]*", text):
        return False
    rest = PLACEHOLDER.sub(" ", text)
    rest = re.sub(r"\\[ntr\"\\]", " ", rest)
    if not PROSE.search(rest):
        return False
    # CamelCase class or node names ("TabContainer", "Socket_Muzzle*").
    if re.fullmatch(r"[A-Za-z0-9_*]+", rest.strip()) and not ui_line:
        return False
    if ui_line:
        return True
    first = re.search(r"[A-Za-z]", rest)
    return bool(first) and first.group(0).isupper() and len(PROSE.findall(rest)) >= 2


def check_scripts(keys):
    errors = []
    used = set()
    for path in scripts():
        rel = os.path.relpath(path, ROOT)
        with open(path, encoding="utf-8") as handle:
            lines = handle.read().split("\n")
        in_doc = False
        for number, line in enumerate(lines, start=1):
            stripped = line.strip()
            if stripped.startswith('"""'):
                in_doc = not (stripped.count('"""') >= 2) and not in_doc
                continue
            if in_doc or stripped.startswith("#"):
                continue
            code = code_part(line)
            for match in STRING.finditer(code):
                prefix, text = match.group(1), match.group(2)
                if prefix:
                    continue
                if KEY.match(text):
                    used.add(text)
                    if text not in keys:
                        errors.append(f"{rel}:{number}: key {text} is in no locale/*.csv")
                    continue
                if MARKER in line or LOGGING.search(code) or NODE_NAMES.search(code):
                    continue
                if is_prose(text, bool(UI_LINE.search(code))):
                    errors.append(f'{rel}:{number}: hard-coded text "{text}"')
    return used, errors


def main():
    keys, errors = load_tables()
    used, script_errors = check_scripts(keys)
    errors += script_errors
    for error in errors:
        print(error)
    if "--unused" in sys.argv:
        for key in sorted(set(keys) - used):
            print(f"unused? {key} ({keys[key]})")
    print(f"l10n check: {len(keys)} keys, {len(used)} used literally, {len(errors)} problem(s)")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
