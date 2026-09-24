"""Fill keys missing from non-reference locales with the English text.

The fork only maintains `en` and `es` by hand; every other locale falls back to
English for new strings so `bun run check:translations` stays green.
Usage: python scripts/fill-locales.py
"""
import collections
import json
import pathlib

LOCALES = pathlib.Path(__file__).resolve().parent.parent / "src" / "i18n" / "locales"
MAINTAINED = {"en", "es"}


def fill(target, reference):
    changed = False
    for key, value in reference.items():
        if key not in target:
            target[key] = value
            changed = True
        elif isinstance(value, dict) and isinstance(target[key], dict):
            changed |= fill(target[key], value)
    return changed


def load(path):
    return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=collections.OrderedDict)


reference = load(LOCALES / "en" / "translation.json")
for locale_dir in sorted(p for p in LOCALES.iterdir() if p.is_dir()):
    if locale_dir.name in MAINTAINED:
        continue
    path = locale_dir / "translation.json"
    data = load(path)
    if fill(data, reference):
        path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
        print(f"filled {locale_dir.name}")
