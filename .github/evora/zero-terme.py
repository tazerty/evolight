#!/usr/bin/env python3
"""Porte « zéro terme » du build web Evora.

Le nom amont retiré ne doit plus apparaître dans le site servi, binaires compris, sauf
dans LICENSES/ où la licence impose de garder les mentions d'origine telles quelles.
Les copies .br et .gz sont sautées : leur fidélité au fichier d'origine est vérifiée
par la porte de précompression du bundle.

Usage :
  zero-terme.py check <dossier du site>   rouge si le terme reste hors de LICENSES/
  zero-terme.py selftest                  prouve que la porte sait rougir
"""
import os
import re
import sys
import tempfile

TERM = b"c" + b"raft"
# Mots du langage courant sans rapport avec le nom retiré (tables Unicode, noms d'émojis...).
UNRELATED = re.compile(rb"(?:air|space|hover|mine|star|war|witch|love|water|hand|arts? ?(?:&|and|-)? ?)" + TERM)
LICENSES = "LICENSES"
COMPRESSED = (".br", ".gz")


def scan(root):
    faults = []
    allowed = 0
    for current, dirs, files in os.walk(root):
        rel_dir = os.path.relpath(current, root)
        if rel_dir.split(os.sep)[0] == LICENSES:
            dirs[:] = []
            continue
        for name in files:
            if name.endswith(COMPRESSED):
                continue
            path = os.path.join(current, name)
            lowered = open(path, "rb").read().lower()
            start = 0
            while True:
                at = lowered.find(TERM, start)
                if at < 0:
                    break
                start = at + len(TERM)
                context = lowered[max(0, at - 24): at + 24]
                if UNRELATED.search(lowered[max(0, at - 12): at + len(TERM)]):
                    allowed += 1
                    continue
                faults.append((os.path.relpath(path, root), context))
    return faults, allowed


def check(root):
    faults, allowed = scan(root)
    print(f"mots sans rapport tolérés : {allowed}")
    for rel, context in faults[:40]:
        print(f"::error::{rel} : {context!r}")
    print(f"occurrences en défaut : {len(faults)}")
    return 1 if faults else 0


def selftest():
    with tempfile.TemporaryDirectory() as top:
        clean = os.path.join(top, "propre")
        os.makedirs(os.path.join(clean, LICENSES))
        with open(os.path.join(clean, "a.js"), "wb") as handle:
            handle.write(b"console.log('air" + TERM + b"')")
        with open(os.path.join(clean, LICENSES, "NOTICE"), "wb") as handle:
            handle.write(b"Art" + TERM + b" Team")
        trapped = os.path.join(top, "piege")
        os.makedirs(trapped)
        with open(os.path.join(trapped, "a.wasm"), "wb") as handle:
            handle.write(b"\0asm photo" + TERM + b"\0")
        ok = not scan(clean)[0] and len(scan(trapped)[0]) == 1
    print("autotest zéro terme :", "vert" if ok else "ROUGE")
    return 0 if ok else 1


if __name__ == "__main__":
    if len(sys.argv) == 2 and sys.argv[1] == "selftest":
        sys.exit(selftest())
    if len(sys.argv) == 3 and sys.argv[1] == "check":
        sys.exit(check(sys.argv[2]))
    print(__doc__)
    sys.exit(2)
