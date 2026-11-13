#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Porte « horloge » du build web Evora (exception L4-REV-H, EVORA-REV-72 ; gardes liées, EVORA-REV-74).

Sous wasm32-unknown-unknown, std::time::Instant::now() et std::time::SystemTime::now() paniquent
(« time not implemented on this platform ») : le module s'arrête (RuntimeError: unreachable). C'est
ainsi que la v0.19.0 n'ouvrait plus aucun RAW (EVORA-REV-71). La porte relève, dans les crates
LOCALES compilées par le build web (cargo build -p evolight-web --lib --target wasm32-unknown-unknown,
sans --features), chaque lecture d'horloge de std compilée pour cette cible, et refuse toute lecture
absente de la liste blanche ALLOW, où chaque site gardé est justifié (profiling(), garde d'exécution,
appelant natif seulement). Chaque entrée est liée au code qui la garde :
  - « before » : jeton de garde exigé dans la fonction englobante du site, avant lui ;
  - « elsewhere » : jeton exigé dans une fonction nommée du même fichier (garde portée ailleurs) ;
  - « prints » : empreinte du corps de la fonction englobante de chaque site.
Jetons et empreintes se lisent sur le code, commentaires neutralisés et espaces ignorés. Est rouge
une entrée qui ne couvre plus exactement ses sites, dont un jeton a disparu ou dont une fonction
englobante a changé : toute retouche d'une fonction admise force sa relecture. Ce lien ne va pas
plus loin que le fichier : une garde portée par un appelant d'un autre fichier (appelant natif,
système de fichiers ou modèles absents du navigateur) n'est vue que si la fonction du site change.

Méthode, sans cargo ni dépendance (Python 3.11 ou plus, pour tomllib) :
  1. graphe des crates locales depuis les Cargo.toml : features unifiées à point fixe, dépendances
     normales et dépendances de cible évaluées pour wasm32 (ni dev, ni build). Avec --tree, la sortie
     de « cargo tree -p evolight-web --target wasm32-unknown-unknown -e normal,features » (porte E7)
     sert de contre-épreuve : une crate locale du vrai graphe que la porte n'a pas lue est rouge ;
  2. arbre des modules depuis src/lib.rs ; #[cfg] des `mod`, des items, des blocs, des bras et des
     instructions évalués pour wasm32 (test et debug_assertions faux ; un nom de cfg inconnu compte
     comme vrai, et un attribut entre parenthèses ou crochets est ignoré : le doute rend compilé) ;
     commentaires et chaînes neutralisés ;
  3. chemin de chaque appel résolu par les `use` du fichier : un `Instant::now` nu qui n'est pas
     importé de web_time compte comme une horloge de std (le doute rend std).
Limites (ce que la porte ne voit pas ; à relire à chaque montée, UPSTREAM.md d'Evora, L4-REV-H) :
  - crates tierces non lues (leurs sources ne sont pas dans le dépôt) ; include!() et modules nés
    d'une macro non suivis ;
  - formes d'appel non relevées : `<std::time::Instant>::now()`, alias de type
    (`type Clock = std::time::Instant;` puis `Clock::now()`) ;
  - lectures d'horloge faites à l'intérieur de std, et autres API de std non prises en charge sous
    wasm32 : Receiver::recv_timeout, Condvar::wait_timeout, thread::park_timeout, thread::sleep,
    Condvar::wait, SystemTime::elapsed() d'une date qui ne vient pas de now() (UNIX_EPOCH + d),
    env::temp_dir, process::id. Deux recv_timeout sont compilés aujourd'hui pour le web, non atteints :
    preview/src/pool.rs (son seul appelant compilé pour le web, wait_for_worker d'ui-egui/src/render.rs,
    sort sous cfg!(target_arch = "wasm32")) et engine/src/import/files.rs (arguments de lancement de
    l'application de bureau). Pour ces API, la seule preuve en CI est le témoin RAW de la fumée, qui
    ne parcourt qu'un chemin : un DNG synthétique non compressé, sans aperçu intégré.

Usage :
  clock-guards.py check <dépôt> [--tree <sortie de cargo tree>]   rouge si une lecture n'est pas admise
  clock-guards.py list <dépôt>                                     inventaire, compilés et exclus
  clock-guards.py selftest                                         prouve que la porte sait rougir
"""
import hashlib
import os
import re
import sys
import tempfile
from collections import namedtuple

try:
    import tomllib
except ModuleNotFoundError:  # Python < 3.11
    sys.exit("clock-guards.py : Python 3.11 ou plus requis (tomllib)")

WEB_CRATE = "evolight-web"
WEB_TRIPLE = "wasm32-unknown-unknown"

# Liste blanche. Chaque lecture d'horloge de std compilée pour le web doit tomber sous UNE entrée de
# son fichier (`file`, et `text` contenu dans la ligne) ; chaque entrée couvre exactement ses sites, un
# par empreinte de `prints`. Ses gardes sont liées au code (voir l'en-tête) :
#   guard      nature de la garde (profiling(), garde d'exécution, appelant natif) ;
#   before     jeton exigé dans la fonction englobante, avant le site (None : garde portée ailleurs) ;
#   elsewhere  (fonction, jeton) : jeton exigé dans le corps de cette fonction du même fichier ;
#   prints     « fonction:empreinte » du corps de la fonction englobante, une par site (`check` et
#              `list` les affichent). Une empreinte changée se reporte ici seulement après relecture
#              de la garde.
Allow = namedtuple("Allow", "file text guard before elsewhere prints why")
# availability.rs : `background` ne devient Some que par run_in_background(), qui sort sous wasm32 ;
# frame.rs d'ui-egui l'appelle aussi dans le build web.
WASM_BACKGROUND = ("run_in_background", 'if cfg!(target_arch = "wasm32") { return; } self.background = Some(wake);')
ALLOW = [
    # EVORA-REV-112 : empreintes de colour_noise et de render_impl reportées après relecture. Le rendu
    # d'Evora y entre en une seule retouche : dans colour_noise, le champ `evora1: None` du plan des
    # tuiles ; dans render_impl, le pont de scène à la fin de l'étage linéaire (`plan.evora1`) et
    # l'argument de plus de local::prepare. Aucun site d'horloge ajouté, déplacé ni retiré ; chacun
    # reste derrière sa garde `profiling()` (jetons « before » inchangés).
    Allow("crates/pipeline/src/lib.rs", "profiling().then(std::time::Instant::now)", "profiling()",
          "profiling().then(", None, ("colour_noise:c1cc208f96ec", "render_impl:cc5653b7717c"),
          "rendu : modèle de bruit couleur et render_impl ; profiling() lit EVOLIGHT_PROFILE, absent du navigateur : None, l'horloge n'est pas lue"),
    Allow("crates/pipeline/src/lib.rs", "*t = std::time::Instant::now();", "profiling()",
          "if let Some(t) = t {", None, ("render_impl:cc5653b7717c",),
          "render_impl, fermeture lap : seulement dans `if let Some(t)`, None hors profilage"),
    Allow("crates/pipeline/src/lib.rs", "let t = std::time::Instant::now();", "profiling()",
          "if !profiling() { return f(); }", None, ("timed:29b7541a465c",),
          "timed() : `if !profiling() { return f(); }` avant l'horloge"),
    Allow("crates/pipeline/src/local.rs", "crate::profiling().then(std::time::Instant::now)", "profiling()",
          "crate::profiling().then(", None, ("apply_nr:fad6355460f8", "apply_nr:fad6355460f8"),
          "apply_nr, réduction du bruit de luminance puis de couleur"),
    Allow("crates/pipeline/src/nrc_joint.rs", "crate::profiling().then(std::time::Instant::now)", "profiling()",
          "crate::profiling().then(", None, ("denoise_with:977178e82dee",),
          "denoise_with, réduction conjointe du bruit couleur"),
    Allow("crates/pipeline/src/nrc_joint.rs", "*t = Some(std::time::Instant::now());", "profiling()",
          "if let Some(t0) = t {", None, ("denoise_with:977178e82dee",),
          "denoise_with, fermeture lap : seulement dans `if let Some(t0)`, None hors profilage"),
    Allow("crates/engine/src/availability.rs", "let now = std::time::Instant::now();", "garde d'exécution",
          "let Some(wake) = &self.background else { return Some((self.probe)(path)); };", WASM_BACKGROUND, ("exists:ab4a500260c0",),
          "exists() rend avant l'horloge tant que background est None ; run_in_background() sort sous cfg!(target_arch = \"wasm32\")"),
    Allow("crates/engine/src/availability.rs", "st.known.insert(path, (ok, std::time::Instant::now()));", "garde d'exécution",
          None, WASM_BACKGROUND, ("spawn_worker:f8adb43095bc",),
          "fil lc-availability de spawn_worker(), lancé par start() après `let Some(wake) = &self.background else` (exists(), sweep_checked()) : jamais sans run_in_background()"),
    Allow("crates/engine/src/cmd/face_detect.rs", "let started = Instant::now();", "garde d'exécution",
          "let detector = detector(s).map_err(|e| e.into_error(C))?;", None, ("detect:8419b6ffe174",),
          "faces.detect : `detector(s)?` échoue avant (face_models_dir absent du build web, « the desktop app does »)"),
    Allow("crates/engine/src/guard.rs", "std::time::SystemTime::now()", "appelant natif",
          None, None, ("install_hook:f1226b8af7aa",),
          "install_hook(), appelée par l'application de bureau seulement ; le web pose son propre crochet (safety.rs)"),
    Allow("crates/engine/src/import/files.rs", "let now = std::time::Instant::now();", "appelant natif",
          None, None, ("launch_deadline:b5c4c330128f",),
          "launch_deadline(), arguments de lancement de l'application de bureau seulement"),
    Allow("crates/engine/src/import/files.rs", "deadline.saturating_duration_since(std::time::Instant::now())", "appelant natif",
          None, None, ("launch_path_until:e18212d2e059",),
          "launch_path_until(), arguments de lancement de l'application de bureau seulement"),
    Allow("crates/preview/src/disk.rs", "let now = std::time::SystemTime::now();", "garde d'exécution",
          "let Some((p, bytes)) = self.read(key) else {", None, ("get:2b9bc48c884b",),
          "DiskCache::get : seulement après une lecture réussie du cache disque, impossible sans système de fichiers"),
    Allow("crates/ui-egui/src/headless.rs", "Instant::now()", "appelant natif",
          None, None, ("settle:4ce8413887fa", "step_until:672e52488756", "request:03c7e0c8fdb6", "request:03c7e0c8fdb6"),
          "Headless (hôte sans fenêtre de la CLI et des tests) : le build web ne l'emploie que dans ses tests"),
]

# --- cfg ----------------------------------------------------------------------------------------

TARGET = {
    "target_arch": "wasm32", "target_family": "wasm", "target_os": "unknown", "target_vendor": "unknown",
    "target_env": "", "target_pointer_width": "32", "target_endian": "little", "panic": "abort",
}
FLAGS = {"unix": False, "windows": False, "test": False, "debug_assertions": False, "miri": False, "doc": False,
         "doctest": False, "docsrs": False, "clippy": False, "fuzzing": False, "coverage": False}
TARGET_FEATURES = {"mutable-globals", "sign-ext", "bulk-memory", "multivalue", "reference-types", "nontrapping-fptoint"}
TOKEN = re.compile(r'[A-Za-z_][A-Za-z0-9_]*|"(?:\\.|[^"\\])*"|[(),=]')


def cfg_eval(expr, feats=frozenset()):
    """Évalue une expression cfg (« cfg(...) » ou son contenu) pour wasm32-unknown-unknown.
    Rend True, False ou None (inconnu)."""
    m = re.fullmatch(r"\s*cfg\s*\((.*)\)\s*", expr, re.S)
    toks = TOKEN.findall(m.group(1) if m else expr)
    pos = 0

    def peek():
        return toks[pos] if pos < len(toks) else None

    def take():
        nonlocal pos
        if pos >= len(toks):
            raise ValueError("cfg tronqué")
        pos += 1
        return toks[pos - 1]

    def pred():
        name = take()
        if name in ("all", "any", "not") and peek() == "(":
            take()
            args = []
            while peek() not in (")", None):
                args.append(pred())
                if peek() == ",":
                    take()
            take()
            if name == "not":
                return None if not args or args[0] is None else not args[0]
            if name == "all":
                return False if False in args else (None if None in args else True)
            return True if True in args else (None if None in args else False)
        if peek() == "=":
            take()
            val = take().strip('"')
            if name == "feature":
                return val in feats
            if name == "target_feature":
                return val in TARGET_FEATURES
            if name == "target_has_atomic":
                return True
            if name in TARGET:
                return TARGET[name] == val
            return None
        return FLAGS.get(name)

    try:
        return pred()
    except (ValueError, IndexError):
        return None


def excluded(cfgs, feats):
    """Exclu du build web seulement si un cfg y est faux à coup sûr."""
    return any(cfg_eval(c, feats) is False for c in cfgs)


# --- graphe des crates locales -------------------------------------------------------------------

def load_toml(path):
    with open(path, "rb") as f:
        return tomllib.load(f)


def manifests_of(root):
    found = {}
    for top in ("crates", "apps", os.path.join("vendor", "evo-libs")):
        base = os.path.join(root, top)
        if not os.path.isdir(base):
            continue
        for d in sorted(os.listdir(base)):
            p = os.path.join(base, d, "Cargo.toml")
            if os.path.isfile(p):
                data = load_toml(p)
                if "package" in data:
                    found[os.path.normpath(os.path.join(top, d))] = data
    return found


def target_ok(t):
    if t is None:
        return True
    if t.strip().startswith("cfg"):
        return cfg_eval(t) is not False
    return t.strip() == WEB_TRIPLE


def web_graph(root):
    """{nom de crate locale: (dossier relatif, features actives)} du build web."""
    ws = load_toml(os.path.join(root, "Cargo.toml"))
    wsdeps = ws.get("workspace", {}).get("dependencies", {})
    by_dir = manifests_of(root)
    name_of = {d: data["package"]["name"] for d, data in by_dir.items()}
    dir_of = {n: d for d, n in name_of.items()}
    if WEB_CRATE not in dir_of:
        raise SystemExit(f"crate {WEB_CRATE} introuvable sous {root}")

    def norm(key, spec, mdir):
        spec = {"version": spec} if isinstance(spec, str) else dict(spec)
        base = os.path.join(root, mdir)
        if spec.get("workspace"):
            w = wsdeps.get(key, {})
            w = {"version": w} if isinstance(w, str) else dict(w)
            feats = list(w.get("features", [])) + list(spec.get("features", []))
            default = spec.get("default-features", w.get("default-features", True))
            spec = {**w, "features": feats, "optional": spec.get("optional", False), "default-features": default}
            base = root
        local, rel = None, None
        if spec.get("path"):
            rel = os.path.relpath(os.path.normpath(os.path.join(base, spec["path"])), root)
            local = name_of.get(rel)
        return {"key": key, "local": local, "path": rel, "optional": spec.get("optional", False),
                "default": spec.get("default-features", True), "features": list(spec.get("features", []))}

    def deps(name):
        mdir = dir_of[name]
        data = by_dir[mdir]
        out = [norm(k, v, mdir) for k, v in data.get("dependencies", {}).items()]
        for t, sec in data.get("target", {}).items():
            if target_ok(t):
                out += [norm(k, v, mdir) for k, v in sec.get("dependencies", {}).items()]
        return out

    lost = set()  # dépendances locales (par chemin) dont la porte ne connaît pas le manifeste
    defs0 = by_dir[dir_of[WEB_CRATE]].get("features", {})
    active = {WEB_CRATE: {"default"} if "default" in defs0 else set()}
    changed = True
    while changed:
        changed = False
        for name in list(active):
            defs = by_dir[dir_of[name]].get("features", {})
            feats = active[name]
            stack = list(feats)
            while stack:
                for sub in defs.get(stack.pop(), []):
                    if "/" not in sub and not sub.startswith("dep:") and sub in defs and sub not in feats:
                        feats.add(sub)
                        stack.append(sub)
            enabled, asked = set(), {}
            for f in feats:
                for sub in defs.get(f, []):
                    if sub.startswith("dep:"):
                        enabled.add(sub[4:])
                    elif "/" in sub:
                        dk, df = sub.split("/", 1)
                        if not dk.endswith("?"):
                            enabled.add(dk)
                        asked.setdefault(dk.rstrip("?"), set()).add(df)
                    elif sub not in defs:
                        enabled.add(sub)
                if f not in defs:
                    enabled.add(f)
            for dp in deps(name):
                if dp["optional"] and dp["key"] not in enabled:
                    continue
                if not dp["local"]:
                    if dp["path"]:
                        lost.add(f"{name} -> {dp['path']}")
                    continue
                req = set(dp["features"]) | asked.get(dp["key"], set())
                tdefs = by_dir[dir_of[dp["local"]]].get("features", {})
                if dp["default"] and "default" in tdefs:
                    req.add("default")
                req &= set(tdefs)
                tgt = dp["local"]
                if tgt not in active:
                    active[tgt] = set(req)
                    changed = True
                elif not req <= active[tgt]:
                    active[tgt] |= req
                    changed = True
    return {n: (dir_of[n], frozenset(f)) for n, f in sorted(active.items())}, sorted(lost)


def tree_locals(text):
    """Crates locales (chemin entre parenthèses) de la sortie de cargo tree."""
    out = set()
    for line in text.splitlines():
        m = re.match(r"\s*([A-Za-z0-9_-]+) v\S+ \((/[^)]*)\)", line)
        if m:
            out.add(m.group(1))
    return out


# --- lecture des sources -------------------------------------------------------------------------

def blank(chars, a, b):
    for k in range(a, min(b, len(chars))):
        if chars[k] != "\n":
            chars[k] = " "


LEX = re.compile(r'//|/\*|[bc]?r#*"|"|\'')
CHAR = re.compile(r"'(\\.|\\u\{[0-9a-fA-F]+\}|\\x[0-9a-fA-F]{2}|[^\\'\n])'")
IDENT = re.compile(r"[A-Za-z0-9_]")


def lex(src, keep_strings=False):
    """Le source, commentaires (et, sauf keep_strings, contenus de chaînes et de caractères) remplacés
    par des espaces (offsets gardés)."""
    out = list(src)

    def hide(a, b):
        if not keep_strings:
            blank(out, a, b)

    i, n = 0, len(src)
    while True:
        m = LEX.search(src, i)
        if not m:
            break
        i, tok = m.start(), m.group(0)
        if tok == "//":
            j = src.find("\n", i)
            j = n if j < 0 else j
            blank(out, i, j)
            i = j
        elif tok == "/*":
            depth, j = 1, i + 2
            while j < n and depth:
                if src.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif src.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            blank(out, i, j)
            i = j
        elif tok.endswith('"') and tok != '"':
            if i > 0 and IDENT.match(src, i - 1):
                i += 1  # fin d'identifiant (« for" » n'existe pas) : on avance d'un caractère
                continue
            close = '"' + tok[tok.index("r") + 1:-1]
            end = src.find(close, m.end())
            end = n if end < 0 else end
            hide(m.end(), end)
            i = end + len(close)
        elif tok == '"':
            j = i + 1
            while j < n and src[j] != '"':
                j += 2 if src[j] == "\\" else 1
            hide(i + 1, j)
            i = j + 1
        else:
            c = CHAR.match(src, i)
            if c:
                hide(i + 1, c.end() - 1)
                i = c.end()
            else:
                i += 1  # durée de vie (« 'a »)
    return "".join(out)


def squash(text):
    """Le texte sans aucun blanc : jetons et empreintes ne dépendent ni de la mise en forme, ni des
    commentaires (déjà neutralisés par lex)."""
    return re.sub(r"\s+", "", text)


def fingerprint(name, body):
    """Empreinte (12 chiffres hexadécimaux) d'une fonction : son nom et son corps, sans blancs."""
    return hashlib.sha256((name + "\n" + squash(body)).encode("utf-8")).hexdigest()[:12]


ATTR = re.compile(r"#(!?)\s*\[\s*cfg\s*\(")
BRACKETS = re.compile(r"[()\[\]]")


def cfgs_at_depth0(clean, src, a, b):
    """#[cfg(...)] et #![cfg(...)] de [a, b) hors parenthèses et crochets (contenu lu dans src)."""
    outer, inner = [], []
    if clean.find("cfg", a, b) < 0:
        return outer, inner
    for m in ATTR.finditer(clean, a, b):
        depth = 0
        for br in BRACKETS.finditer(clean, a, m.start()):
            depth += 1 if br.group(0) in "([" else -1
        if depth > 0:
            continue
        j, d = m.end(), 1
        while j < len(clean) and d:
            d += {"(": 1, ")": -1}.get(clean[j], 0)
            j += 1
        (inner if m.group(1) else outer).append("cfg(" + src[m.end():j - 1] + ")")
    return outer, inner


def use_map(clean):
    """{nom local: [chemins complets]} des `use` du fichier (toutes portées), globs sous « * »."""
    names = {}
    for m in re.finditer(r"\buse\s+([^;]+);", clean):
        toks = re.findall(r"[A-Za-z_][A-Za-z0-9_]*|::|[{},*]", m.group(1))

        def tree(i, prefix):
            res, segs = [], list(prefix)
            while i < len(toks):
                t = toks[i]
                if t == "::":
                    i += 1
                elif t == "{":
                    i += 1
                    while i < len(toks) and toks[i] != "}":
                        sub, i = tree(i, segs)
                        res += sub
                        if i < len(toks) and toks[i] == ",":
                            i += 1
                    return res, i + 1
                elif t == "*":
                    return res + [(segs, "*")], i + 1
                elif t in (",", "}"):
                    return res, i
                else:
                    segs.append(t)
                    i += 1
                    if i < len(toks) and toks[i] == "::":
                        continue
                    alias = segs[-1]
                    if i + 1 < len(toks) and toks[i] == "as":
                        alias, i = toks[i + 1], i + 2
                    if segs[-1] == "self":
                        segs = segs[:-1]
                        alias = segs[-1] if alias == "self" and segs else alias
                    return res + [(segs, alias)], i
            return res, i

        for segs, alias in tree(0, [])[0]:
            names.setdefault(alias, []).append(segs)
    return names


PATH_BEFORE = re.compile(r"(?:::\s*)?(?:[A-Za-z_][A-Za-z0-9_]*\s*::\s*)*$")


def clock_hits(clean, uses):
    """[(début du chemin, chemin, dernier segment)] des appels now() d'Instant, de SystemTime ou d'un alias
    importé de l'un d'eux, et des UNIX_EPOCH.elapsed() (qui lisent SystemTime::now())."""
    names = {"Instant", "SystemTime"} | {a for a, paths in uses.items() if any(p[-1:] in (["Instant"], ["SystemTime"]) for p in paths)}
    alt = "|".join(sorted(re.escape(n) for n in names))
    hits = []
    for m in re.finditer(r"\b(" + alt + r")\s*::\s*now\b|\b(UNIX_EPOCH)\s*\.\s*elapsed\s*\(", clean):
        chunk = clean[max(0, m.start() - 240):m.start()]
        pm = PATH_BEFORE.search(chunk)
        path = pm.group(0) if pm else ""
        hits.append((m.start() - len(path), path, m.group(1) or m.group(2)))
    return hits


def origin(path, last, uses):
    """« std », « web_time » ou « ? » pour le chemin d'un appel (segments avant now/UNIX_EPOCH)."""
    segs = [s for s in re.split(r"\s*::\s*", path.strip()) if s] + [last]
    if segs[0] in ("std", "core", "web_time"):
        cands = [segs]
    elif segs[0] in uses:
        cands = [full + segs[1:] for full in uses[segs[0]]]
    else:
        cands = [full + segs for full in uses.get("*", [])]
    kinds = set()
    for full in cands:
        if full[:1] in (["std"], ["core"]) and full[1:2] == ["time"]:
            kinds.add("std")
        elif full[:1] == ["web_time"]:
            kinds.add("web_time")
        else:
            kinds.add("?")
    if "std" in kinds:
        return "std"
    return "web_time" if kinds == {"web_time"} else "?"


STRUCT = re.compile(r"[(){}\[\];,]")
MOD_DECL = re.compile(r"\bmod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;\s*$")
MOD_HEAD = re.compile(r"\bmod\s+([A-Za-z_][A-Za-z0-9_]*)\s*$")
BLOCK_WORDS = re.compile(r"\b(fn|impl|trait|mod|if|else|while|for|loop|unsafe|async|move|extern|macro_rules|const|static|where)\b")
FN_HEAD = re.compile(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)")


def comma_body(header):
    """Bloc dont les éléments se séparent par des virgules (bras de match, champs et variantes,
    littéral de structure) : un #[cfg] n'y garde que son élément. Ailleurs (corps de fonction,
    impl, module), une virgule hors parenthèses n'est qu'un séparateur de génériques."""
    h = header.strip()
    if re.search(r"\bmatch\b", h) or re.search(r"\b(enum|struct|union)\s+[A-Za-z_]", h):
        return True
    return bool(re.search(r"[A-Za-z0-9_]$", h)) and not BLOCK_WORDS.search(h)


def scan_file(path, feats, inherited):
    """Rend (sites, modules déclarés, cfg du fichier, fonctions) ; chaque site porte ses cfg complets,
    son offset, sa fonction englobante (nom, offset de son « { ») et l'empreinte de son corps."""
    with open(path, encoding="utf-8") as f:
        src = f.read()
    clean = lex(src)
    uses = use_map(clean)
    events = [(at, 0, (p, last)) for at, p, last in clock_hits(clean, uses)]
    events += [(m.start(), 1, m.group(0)) for m in STRUCT.finditer(clean)]
    events.sort(key=lambda e: (e[0], e[1]))
    sites, mods = [], []
    frames = []  # (cfgs du bloc, nom du module en ligne ou None, bloc à virgules, fonction ou None)
    fns = []  # fonctions du fichier : {"name", "open", "close"} (offsets de leur « { » et de leur « } »)
    file_cfgs = list(inherited)
    depth, depths, last = 0, [], 0
    lines = src.splitlines()
    scope = lambda: frames[-1][0] if frames else file_cfgs  # noqa: E731
    for i, kind, val in events:
        if kind == 0:
            outer, inner = cfgs_at_depth0(clean, src, last, i)
            chain = file_cfgs + [x for fr in frames for x in fr[0]] + inner + outer
            line = src.count("\n", 0, i) + 1
            fn = next((fr[3] for fr in reversed(frames) if fr[3]), None)
            sites.append({"line": line, "text": lines[line - 1].strip(), "kind": origin(val[0], val[1], uses),
                          "cfgs": chain, "compiled": not excluded(chain, feats), "offset": i, "_fn": fn})
        elif val in "([":
            depth += 1
        elif val in ")]":
            depth = max(0, depth - 1)
        elif val == "{":
            outer, inner = cfgs_at_depth0(clean, src, last, i)
            scope().extend(inner)
            mm = MOD_HEAD.search(clean, last, i)
            fm = FN_HEAD.search(clean, last, i)
            fn = {"name": fm.group(1), "open": i, "close": len(src)} if fm else None
            if fn:
                fns.append(fn)
            frames.append((outer, mm.group(1) if mm else None, comma_body(clean[last:i]), fn))
            depths.append(depth)
            depth, last = 0, i + 1
        elif val == "}":
            if frames:
                fn = frames.pop()[3]
                if fn:
                    fn["close"] = i
                depth = depths.pop()
            last = i + 1
        elif depth == 0 and (val == ";" or frames and frames[-1][2]):  # « ; », ou « , » d'un bloc à virgules
            seg_outer, seg_inner = cfgs_at_depth0(clean, src, last, i + 1)
            scope().extend(seg_inner)
            mm = MOD_DECL.search(clean, last, i + 1) if val == ";" else None
            if mm:
                pm = re.search(r'#\s*\[\s*path\s*=\s*"([^"]+)"', src[last:i + 1])
                chain = file_cfgs + [x for fr in frames for x in fr[0]] + seg_outer
                inline = [fr[1] for fr in frames if fr[1]]
                mods.append((mm.group(1), pm.group(1) if pm else None, inline, chain))
            last = i + 1
    code = lex(src, keep_strings=True) if sites else ""
    for s in sites:
        fn = s.pop("_fn")
        s["fn"], s["fn_open"] = (fn["name"], fn["open"]) if fn else (None, None)
        s["print"] = fingerprint(fn["name"], code[fn["open"] + 1:fn["close"]]) if fn else None
    return sites, mods, file_cfgs, fns


def mod_candidates(parent, name, pval, inline):
    d, base = os.path.dirname(parent), os.path.basename(parent)
    owner = base in ("lib.rs", "mod.rs", "main.rs")
    sub = os.path.join(d, *inline) if owner else os.path.join(d, base[:-3], *inline)
    if pval:
        anchor = os.path.join(d, *inline) if inline else d
        return [os.path.normpath(os.path.join(anchor, pval))]
    return [os.path.join(sub, name + ".rs"), os.path.join(sub, name, "mod.rs")]


def scan(root):
    """Tous les sites d'horloge des crates locales du build web, et les fonctions de chaque fichier lu."""
    graph, lost = web_graph(root)
    sites, missing, files = [], [f"dépendance locale hors de crates/, apps/ et vendor/evo-libs/ : {x}" for x in lost], 0
    spans = {}
    for crate, (cdir, feats) in graph.items():
        lib = os.path.join(root, cdir, "src", "lib.rs")
        if not os.path.isfile(lib):
            continue
        queue, seen = [(lib, [])], set()
        while queue:
            f, chain = queue.pop()
            if f in seen:
                continue
            seen.add(f)
            files += 1
            found, mods, fcfgs, spans[os.path.relpath(f, root)] = scan_file(f, feats, chain)
            for s in found:
                s.update({"crate": crate, "file": os.path.relpath(f, root)})
                sites.append(s)
            for name, pval, inline, mchain in mods:
                cands = mod_candidates(f, name, pval, inline)
                target = next((c for c in cands if os.path.isfile(c)), None)
                if target is None:
                    if not excluded(mchain, feats):
                        missing.append(f"module compilé introuvable : {os.path.relpath(f, root)} : mod {name}")
                    continue
                queue.append((target, mchain))
    return graph, sites, missing, files, spans


def guard_problems(a, s, where, code, fns):
    """Problèmes de garde d'un site admis par l'entrée a : jetons « before » et « elsewhere » absents."""
    out = []
    if a.before is not None:
        seen = code[s["fn_open"] + 1:s["offset"]] if s["fn_open"] is not None else ""
        if squash(a.before) not in squash(seen):
            out.append(f"garde absente : {where} : « {a.before} » introuvable avant le site dans fn {s['fn'] or '(aucune)'}")
    if a.elsewhere is not None:
        name, token = a.elsewhere
        bodies = [code[f["open"] + 1:f["close"]] for f in fns if f["name"] == name]
        if not any(squash(token) in squash(b) for b in bodies):
            out.append(f"garde absente : {where} : « {token} » introuvable dans fn {name} de {a.file}")
    return out


def judge(root, allow, tree_text=None):
    """Rend (problèmes, lignes du rapport, sites)."""
    graph, sites, missing, files, spans = scan(root)
    problems, report = [], []
    report.append(f"crates locales du build web : {len(graph)} ({', '.join(graph)})")
    report.append(f"fichiers lus : {files} ; sites d'horloge relevés : {len(sites)}")
    if WEB_CRATE not in graph or "evolight-engine" not in graph:
        problems.append("graphe du build web vide ou changé de forme (evolight-web et evolight-engine attendus)")
    if tree_text is not None:
        real = tree_locals(tree_text)
        if not real:
            problems.append("contre-épreuve : aucune crate locale dans la sortie de cargo tree (vide ou changée)")
        unread = sorted(real - set(graph))
        if unread:
            problems.append(f"contre-épreuve : crates locales du vrai graphe web non lues par la porte : {', '.join(unread)}")
        extra = sorted(set(graph) - real)
        report.append(f"contre-épreuve cargo tree : {len(real)} crates locales ; lues en plus par la porte : {', '.join(extra) or 'aucune'}")
    for m in missing:
        problems.append(f"source non lue par la porte : {m}")
    web_time = [s for s in sites if s["compiled"] and s["kind"] == "web_time"]
    std = [s for s in sites if s["compiled"] and s["kind"] != "web_time"]
    out = [s for s in sites if not s["compiled"]]
    report.append(f"compilés pour wasm32 : {len(std)} lectures de std (ou d'origine douteuse), {len(web_time)} de web_time ; exclus par cfg : {len(out)}")
    used = [[] for _ in allow]  # « fonction:empreinte » relevées, entrée par entrée
    codes = {}  # code des fichiers admis (commentaires neutralisés, chaînes gardées), lu une fois
    for s in std:
        fits = [k for k, a in enumerate(allow) if a.file == s["file"] and a.text in s["text"]]
        where = f"{s['file']}:{s['line']} ({s['crate']}, {s['kind']})"
        if not fits:
            problems.append(f"lecture d'horloge de std sans garde admise : {where} : {s['text']}")
            continue
        if len(fits) > 1:
            problems.append(f"site couvert par plusieurs entrées de la liste blanche : {where}")
        a = allow[fits[0]]
        used[fits[0]].append(f"{s['fn'] or '-'}:{s['print'] or '-'}")
        if s["file"] not in codes:
            with open(os.path.join(root, s["file"]), encoding="utf-8") as f:
                codes[s["file"]] = lex(f.read(), keep_strings=True)
        problems += guard_problems(a, s, where, codes[s["file"]], spans.get(s["file"], []))
        report.append(f"admis {where} [{a.guard}] fn {s['fn'] or '-'} {s['print'] or '-'} : {s['text']}")
    for k, a in enumerate(allow):
        if len(used[k]) != len(a.prints):
            problems.append(f"liste blanche à relire : {a.file} « {a.text} » couvre {len(used[k])} site(s), {len(a.prints)} annoncé(s)")
        elif sorted(used[k]) != sorted(a.prints):
            problems.append(f"liste blanche à relire : {a.file} « {a.text} » : corps de fonction englobante changé "
                            f"(empreintes relevées {', '.join(sorted(used[k]))} ; annoncées {', '.join(sorted(a.prints))}) : "
                            "relire la garde, puis reporter l'empreinte")
    return problems, report, sites


# --- auto-test ------------------------------------------------------------------------------------

def write(base, rel, text):
    p = os.path.join(base, rel)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, "w", encoding="utf-8") as f:
        f.write(text)


SELF_ALLOW = [
    Allow("crates/a/src/lib.rs", "profiling().then(std::time::Instant::now)", "profiling()",
          "profiling().then(", None, ("guarded:11c571407a22",), "auto-test : garde dans la ligne"),
    Allow("crates/a/src/lib.rs", "let t = std::time::Instant::now();", "profiling()",
          "if !profiling() { return f(); }", None, ("timed:9e9c634d2097",), "auto-test : garde avant le site, dans la fonction"),
    Allow("crates/a/src/lib.rs", "Some(std::time::Instant::now())", "garde d'exécution",
          "if !background(on) { return None; }", ("background", 'if cfg!(target_arch = "wasm32") { return false; }'),
          ("probe:4d4bcb7e681c",), "auto-test : garde portée par une autre fonction du fichier"),
]

LIB_A = """#![allow(dead_code)]
mod clock;
#[cfg(test)]
mod tests;
#[cfg(not(target_arch = "wasm32"))]
mod native_only;
#[cfg(feature = "extra")]
mod extra;
pub fn profiling() -> bool { false }
pub fn guarded() -> Option<std::time::Instant> { profiling().then(std::time::Instant::now) }
pub fn fine() -> web_time::Instant { web_time::Instant::now() }
#[cfg(not(target_arch = "wasm32"))]
pub fn desktop() { let _ = std::time::Instant::now(); }
pub fn arms(x: u8) {
    match x {
        #[cfg(not(target_arch = "wasm32"))]
        0 => { let _ = std::time::SystemTime::now(); }
        _ => {}
    }
}
// std::time::Instant::now() dans un commentaire
const S: &str = "std::time::Instant::now()";
pub fn timed<R>(f: impl FnOnce() -> R) -> R {
    if !profiling() {
        return f();
    }
    let t = std::time::Instant::now();
    let r = f();
    let _ = t;
    r
}
pub fn background(on: bool) -> bool {
    if cfg!(target_arch = "wasm32") {
        return false;
    }
    on
}
pub fn probe(on: bool) -> Option<std::time::Instant> {
    if !background(on) {
        return None;
    }
    Some(std::time::Instant::now())
}
"""
CLOCK_OK = "use web_time::{Duration, Instant as Clock};\npub fn now() -> Clock { Clock::now() }\n"
TRAPS = {
    # alias d'un import de std dans un fichier compilé
    "alias": ("crates/a/src/clock.rs", "use std::time::{Duration, Instant as Clock};\npub fn now() -> Clock { Clock::now() }\n", 2),
    # un cfg sur un paramètre ne garde pas le corps de la fonction
    "parametre": ("crates/a/src/clock.rs", CLOCK_OK + "pub fn p(#[cfg(test)] _a: u8) { let _ = std::time::SystemTime::now(); }\n", 3),
    # un cfg sur un bras ne garde pas le bras suivant (séparé par une virgule)
    "bras": ("crates/a/src/clock.rs", CLOCK_OK + "pub fn b(x: u8) -> u8 { match x {\n #[cfg(test)]\n 0 => 1,\n"
             " _ => { let _ = ::std::time::Instant::now(); 2 }\n} }\n", 6),
    # glob de std::time
    "glob": ("crates/a/src/clock.rs", "use std::time::*;\npub fn g() -> SystemTime { SystemTime::now() }\n", 2),
    # UNIX_EPOCH.elapsed() lit SystemTime::now()
    "epoch": ("crates/a/src/clock.rs", CLOCK_OK + "pub fn e() { let _ = std::time::UNIX_EPOCH.elapsed(); }\n", 3),
    # origine introuvable : le doute rend std
    "douteux": ("crates/a/src/clock.rs", CLOCK_OK + "pub fn d() { let _ = crate::Instant::now(); }\n", 3),
    # #![cfg] d'un module en ligne ne s'étend pas au reste du fichier
    "interne": ("crates/a/src/clock.rs", CLOCK_OK + "mod inner {\n #![cfg(test)]\n fn i() {}\n}\npub fn o() { let _ = std::time::Instant::now(); }\n", 7),
    # feature activée par le paquet web : la crate optionnelle entre dans le graphe
    "feature": ("apps/evolight-web/Cargo.toml", None, None),
}
# Pièges de garde, dans lib.rs : (remplacement, fragments attendus dans les problèmes, nombre de problèmes).
GUARD_TRAPS = {
    # garde retirée de la fonction (le cas de timed() du pipeline) : jeton absent et corps changé
    "garde retirée": (("    if !profiling() {\n        return f();\n    }\n", ""),
                      ["garde absente : crates/a/src/lib.rs:24 ", "dans fn timed", "corps de fonction englobante changé (empreintes relevées timed:"], 2),
    # garde portée par une autre fonction du fichier, retirée : seul le jeton « elsewhere » la voit
    "garde ailleurs retirée": (('    if cfg!(target_arch = "wasm32") {\n        return false;\n    }\n', ""),
                               ["garde absente : crates/a/src/lib.rs:39 ", "introuvable dans fn background"], 1),
    # garde déplacée après le site : le jeton n'est plus avant lui
    "garde déplacée": (("    if !background(on) {\n        return None;\n    }\n    Some(std::time::Instant::now())\n",
                        "    let t = Some(std::time::Instant::now());\n    if !background(on) {\n        return None;\n    }\n    t\n"),
                       ["garde absente : crates/a/src/lib.rs:39 ", "dans fn probe", "corps de fonction englobante changé (empreintes relevées probe:"], 2),
    # corps changé, garde intacte : l'empreinte force la relecture
    "corps changé": (("    let _ = t;\n", "    let _ = t;\n    let _ = 0;\n"), ["corps de fonction englobante changé (empreintes relevées timed:"], 1),
    # témoin : commentaire et blancs ajoutés dans la fonction, le code ne change pas
    "commentaire (témoin vert)": (("    let r = f();\n", "    // rien ne change\n\n    let r =   f();\n"), [], 0),
}


def fixture(base, trap=None):
    write(base, "Cargo.toml", '[workspace]\nmembers = ["crates/*", "apps/*"]\nresolver = "3"\n\n[workspace.dependencies]\n'
          'a = { path = "crates/a" }\nweb-time = "1"\nevolight-engine = { path = "crates/engine" }\n')
    feature = trap == "feature"
    write(base, "apps/evolight-web/Cargo.toml", '[package]\nname = "evolight-web"\nversion = "0.0.0"\n\n[dependencies]\n'
          'a = { workspace = true' + (', features = ["extra"]' if feature else '') + ' }\nevolight-engine = { workspace = true }\n'
          'opt = { path = "../../crates/opt", optional = true }\n\n'
          '[target.\'cfg(not(target_arch = "wasm32"))\'.dependencies]\nnative = { path = "../../crates/native" }\n\n'
          '[dev-dependencies]\ndev = { path = "../../crates/dev" }\n')
    write(base, "apps/evolight-web/src/lib.rs", "pub fn x() {}\n")
    write(base, "crates/a/Cargo.toml", '[package]\nname = "a"\nversion = "0.0.0"\n\n[features]\nextra = []\n\n'
          '[dependencies]\nweb-time = { workspace = true }\n')
    write(base, "crates/a/src/lib.rs", LIB_A)
    write(base, "crates/a/src/clock.rs", CLOCK_OK)
    write(base, "crates/a/src/tests.rs", "fn t() { let _ = std::time::Instant::now(); }\n")
    write(base, "crates/a/src/native_only.rs", "fn n() { let _ = std::time::Instant::now(); }\n")
    write(base, "crates/a/src/extra.rs", "pub fn x() { let _ = std::time::Instant::now(); }\n")
    write(base, "crates/engine/Cargo.toml", '[package]\nname = "evolight-engine"\nversion = "0.0.0"\n')
    write(base, "crates/engine/src/lib.rs", "pub fn e() {}\n")
    for other in ("opt", "native", "dev"):
        write(base, f"crates/{other}/Cargo.toml", f'[package]\nname = "{other}"\nversion = "0.0.0"\n')
        write(base, f"crates/{other}/src/lib.rs", "pub fn o() { let _ = std::time::Instant::now(); }\n")
    if trap and TRAPS[trap][1] is not None:
        write(base, TRAPS[trap][0], TRAPS[trap][1])


def selftest():
    checks = []
    with tempfile.TemporaryDirectory(prefix="evora-clock-") as tmp:
        base = os.path.join(tmp, "temoin")
        fixture(base)
        problems, _, sites = judge(base, SELF_ALLOW)
        compiled = [s for s in sites if s["compiled"] and s["kind"] != "web_time"]
        files = {s["file"] for s in sites}
        checks.append(("témoin : arbre propre accepté (gardes liées admises, cfg exclus, web_time, commentaire et chaîne ignorés)",
                       not problems and len(compiled) == 3 and not files & {"crates/opt/src/lib.rs", "crates/native/src/lib.rs", "crates/dev/src/lib.rs"}))
        excl = sorted((s["file"], s["line"]) for s in sites if not s["compiled"])
        checks.append(("témoin : tests, module natif, feature éteinte, item et bras sous cfg exclus",
                       excl == sorted([("crates/a/src/extra.rs", 1), ("crates/a/src/lib.rs", 13), ("crates/a/src/lib.rs", 17),
                                       ("crates/a/src/native_only.rs", 1), ("crates/a/src/tests.rs", 1)])))
        checks.append(("liste pourrie : une entrée qui ne couvre plus rien est rouge",
                       any("liste blanche à relire" in p for p in judge(base, SELF_ALLOW + [Allow("crates/a/src/lib.rs", "disparu", "x", None, None, ("x:x",), "x")])[0])))
        checks.append(("contre-épreuve : une crate locale du vrai graphe non lue est rouge",
                       any("non lues par la porte" in p for p in judge(base, SELF_ALLOW, "evolight-web v0.0.0 (/x/apps/evolight-web)\n"
                                                                                       "native v0.0.0 (/x/crates/native)\n")[0])))
        checks.append(("contre-épreuve : sortie de cargo tree vide rouge", any("aucune crate locale" in p for p in judge(base, SELF_ALLOW, "")[0])))
        lost = os.path.join(tmp, "chemin")
        fixture(lost)
        write(lost, "crates/a/Cargo.toml", '[package]\nname = "a"\nversion = "0.0.0"\n\n[features]\nextra = []\n\n'
              '[dependencies]\nweb-time = { workspace = true }\nhors = { path = "../../outils/hors" }\n')
        write(lost, "outils/hors/Cargo.toml", '[package]\nname = "hors"\nversion = "0.0.0"\n')
        write(lost, "outils/hors/src/lib.rs", "pub fn h() { let _ = std::time::Instant::now(); }\n")
        checks.append(("dépendance locale hors des dossiers connus : rouge (source non lue)",
                       any("dépendance locale hors de crates/" in p and "outils/hors" in p for p in judge(lost, SELF_ALLOW)[0])))
        for trap, (rel, _, line) in TRAPS.items():
            tdir = os.path.join(tmp, trap)
            fixture(tdir, trap)
            problems = judge(tdir, SELF_ALLOW)[0]
            want = f"crates/a/src/clock.rs:{line} " if line else "crates/a/src/extra.rs:1 "
            hit = [p for p in problems if "sans garde admise" in p]
            checks.append((f"piège « {trap} » : rouge, sur le seul site attendu", len(hit) == 1 and want in hit[0] and len(problems) == 1))
        for n, (trap, ((old, new), wants, count)) in enumerate(GUARD_TRAPS.items()):
            tdir = os.path.join(tmp, f"garde-{n}")
            fixture(tdir)
            assert LIB_A.count(old) == 1, trap
            write(tdir, "crates/a/src/lib.rs", LIB_A.replace(old, new))
            problems = judge(tdir, SELF_ALLOW)[0]
            fits = len(problems) == count and all(any(w in p for p in problems) for w in wants)
            checks.append((f"piège « {trap} » : {count} problème(s) attendu(s)", fits))
    ok = True
    for name, passed in checks:
        print(f"{'OK   ' if passed else 'ÉCHEC'} {name}")
        ok = ok and passed
    print(f"Auto-test clock-guards : {'OK' if ok else 'ÉCHEC'}")
    return ok


# --- entrée ---------------------------------------------------------------------------------------

def main(argv):
    if argv[:1] == ["selftest"]:
        return 0 if selftest() else 1
    if len(argv) < 2 or argv[0] not in ("check", "list"):
        print(__doc__)
        return 2
    root = os.path.abspath(argv[1])
    tree_text = None
    if "--tree" in argv:
        path = argv[argv.index("--tree") + 1]
        try:
            with open(path, encoding="utf-8") as f:
                tree_text = f.read()
        except OSError as err:
            print(f"::error::sortie de cargo tree illisible ({err}) : la contre-épreuve est impossible")
            return 1
    problems, report, sites = judge(root, ALLOW, tree_text)
    for line in report:
        print(line)
    if argv[0] == "list":
        for s in sorted(sites, key=lambda s: (s["file"], s["line"])):
            state = "compilé" if s["compiled"] else "exclu"
            print(f"{state}\t{s['kind']}\t{s['crate']}\t{s['file']}:{s['line']}\tfn {s['fn'] or '-'}\t{s['print'] or '-'}\t{s['text']}\t{' '.join(s['cfgs'])}")
    for p in problems:
        print(f"::error::{p}")
    print("Résultat : ÉCHEC" if problems else "Résultat : OK")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
