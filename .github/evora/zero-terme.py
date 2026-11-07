#!/usr/bin/env python3
"""Porte « zéro terme » du build web Evora.

Le nom amont retiré ne doit plus apparaître dans le site servi, ni dans le contenu des
fichiers, binaires compris, ni dans leur nom, sauf dans LICENSES/ où la licence impose de
garder les mentions d'origine telles quelles.
Il est cherché en ASCII et en UTF-16 : un profil ICC embarqué, par exemple, range ses
textes en UTF-16. Les copies .br et .gz sont sautées : leur fidélité au fichier d'origine
est vérifiée par la porte de précompression du bundle.

Usage :
  zero-terme.py check <dossier du site>   rouge si le terme reste hors de LICENSES/
  zero-terme.py selftest                  prouve que la porte sait rougir
"""
import os
import re
import sys
import tempfile

TERM = b"c" + b"raft"
WIDE = b"\0".join(bytes([letter]) for letter in TERM)
# Mots du langage courant sans rapport avec le nom retiré (tables Unicode, noms d'émojis...).
# Ancré sur la fin : c'est le mot qui s'achève sur l'occurrence examinée qui décide. Le nom de
# l'équipe amont, en un mot ou avec un trait d'union, n'en fait pas partie.
UNRELATED = re.compile(rb"(?:air|space|hover|mine|star|war|witch|love|water|hand|arts? ?(?:&|and) ?)" + TERM + rb"$")
LICENSES = "LICENSES"
COMPRESSED = (".br", ".gz")


def occurrences(lowered):
    """Chaque occurrence : (position, le mot jusqu'au terme compris, débarrassé des NUL d'UTF-16)."""
    for needle, reach in ((TERM, 12), (WIDE, 24)):
        start = 0
        while True:
            at = lowered.find(needle, start)
            if at < 0:
                break
            start = at + len(needle)
            yield at, lowered[max(0, at - reach): at + len(needle)].replace(b"\0", b"")


def scan(root):
    faults = []
    allowed = 0
    for current, dirs, files in os.walk(root):
        rel_dir = os.path.relpath(current, root)
        if rel_dir.split(os.sep)[0] == LICENSES:
            dirs[:] = []
            continue
        for name in files:
            path = os.path.join(current, name)
            rel = os.path.relpath(path, root)
            for _, word in occurrences(os.fsencode(rel).lower()):
                if not UNRELATED.search(word):
                    faults.append((rel, b"nom de fichier"))
            if name.endswith(COMPRESSED):
                continue
            with open(path, "rb") as handle:
                lowered = handle.read().lower()
            for at, word in occurrences(lowered):
                if UNRELATED.search(word):
                    allowed += 1
                    continue
                faults.append((rel, lowered[max(0, at - 24): at + 24]))
    return faults, allowed


def check(root):
    if not os.path.isdir(root) or not any(files for _, _, files in os.walk(root)):
        print(f"::error::dossier du site vide ou absent : {root}")
        return 1
    faults, allowed = scan(root)
    print(f"mots sans rapport tolérés : {allowed}")
    for rel, context in faults[:40]:
        print(f"::error::{rel} : {context!r}")
    print(f"occurrences en défaut : {len(faults)}")
    return 1 if faults else 0


def write(folder, name, data):
    with open(os.path.join(folder, name), "wb") as handle:
        handle.write(data)


def selftest():
    with tempfile.TemporaryDirectory() as top:
        clean = os.path.join(top, "propre")
        os.makedirs(os.path.join(clean, LICENSES))
        write(clean, "a.js", b"console.log('air" + TERM + b"')")
        write(os.path.join(clean, LICENSES), "NOTICE", b"Art" + TERM + b" Team")
        trapped = os.path.join(top, "piege")
        os.makedirs(trapped)
        write(trapped, "moteur.wasm", b"\0asm photo" + TERM + b"\0")
        write(trapped, "equipe.js", b"the Art" + TERM.capitalize() + b" team")
        write(trapped, "tiret.js", b"art-" + TERM)
        write(trapped, "profil.icc", ("Photo" + TERM.decode().capitalize() + " Coated").encode("utf-16-be"))
        write(trapped, "voisin.js", b"air" + TERM + b" photo" + TERM)
        named = TERM.decode() + "_web_bg.wasm"
        write(trapped, named, b"\0asm")
        faults = sorted(rel for rel, _ in scan(trapped)[0])
        expected = sorted(["equipe.js", "moteur.wasm", "profil.icc", "tiret.js", "voisin.js", named])
        ok = not scan(clean)[0] and faults == expected
    print("autotest zéro terme :", "vert" if ok else "ROUGE")
    return 0 if ok else 1


if __name__ == "__main__":
    if len(sys.argv) == 2 and sys.argv[1] == "selftest":
        sys.exit(selftest())
    if len(sys.argv) == 3 and sys.argv[1] == "check":
        sys.exit(check(sys.argv[2]))
    print(__doc__)
    sys.exit(2)
