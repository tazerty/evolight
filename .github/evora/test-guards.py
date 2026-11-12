#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Porte « tests Rust » de la CI d'Evora (lot 1 « REV-TONS » de Revela, EVORA-REV-78).

Juge les journaux de `cargo test` (format par défaut de libtest, sans --nocapture ni -q) contre une
liste fermée de tests attendus (.github/evora/expected-tests.txt). Est rouge :
  - un test attendu absent de son binaire (« n'a pas tourné »), présent plus d'une fois, ignoré ou
    en échec ;
  - un test attendu inscrit au journal des sauts (EVOLIGHT_SKIP_LOG : une ligne JSON par saut,
    champs need, reason, test, binary, location) ;
  - dans un journal marqué « gpu », TOUT saut de besoin « gpu » (le filtre des tests d'accord des
    deux chemins n'en admet aucun : sans adaptateur, chacun passerait sans rien comparer) ;
  - une sonde du noyau graphique sans `"available":true`, ou son témoin (la même sonde sans
    EVOLIGHT_GPU_ALLOW_SOFTWARE) sans `"available":false` ;
  - un binaire attendu sans aucune ligne de résultat ; tout test en échec, attendu ou non.

Liste : une ligne par test attendu, « <journal> <binaire> <chemin du test> » ; lignes « # » et vides
ignorées. Le binaire est la tige de la ligne « Running … (…/deps/<tige>-<16 chiffres hexadécimaux>) »
qui précède ses résultats. Résumé Markdown (tableau par binaire, état de chaque attendu, adaptateur
lu dans la sonde) écrit avec --summary.

Usage :
  test-guards.py check --list FICHIER --log NOM=JOURNAL... [--skips NOM=JOURNAL...] [--gpu-log NOM]
                       [--probe SONDE --probe-off TÉMOIN] [--summary FICHIER]
  test-guards.py selftest
Sans dépendance (Python 3.9 ou plus). Code de sortie 1 si la porte est rouge, 2 sur erreur d'usage.
"""
from __future__ import annotations

import json
import re
import sys
import tempfile
from pathlib import Path

ANSI = re.compile(r'\x1b\[[0-9;]*[A-Za-z]')
RUNNING = re.compile(r'Running\s.*\((?:[^()]*/)?([A-Za-z0-9_]+)-[0-9a-f]{16}(?:\.exe)?\)\s*$')
RESULT = re.compile(r'^test (\S+) \.\.\. (ok|FAILED|ignored)(?:, .*)?$')
TOTALS = re.compile(r'^test result: ')


def read_log(path: Path) -> tuple[dict, dict]:
    """(résultats par (binaire, test) : liste d'états ; nombre de lignes de résultat par binaire)."""
    results, counts = {}, {}
    binary = None
    for raw in path.read_text(encoding='utf-8', errors='replace').splitlines():
        line = ANSI.sub('', raw).rstrip()
        m = RUNNING.search(line)
        if m:
            binary = m.group(1)
            counts.setdefault(binary, 0)
            continue
        m = RESULT.match(line)
        if m and binary is not None:
            results.setdefault((binary, m.group(1)), []).append(m.group(2))
            counts[binary] = counts.get(binary, 0) + 1
    return results, counts


def read_skips(path: Path) -> list[dict]:
    out = []
    if not path.exists():
        return out
    for line in path.read_text(encoding='utf-8', errors='replace').splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            out.append(json.loads(line))
        except json.JSONDecodeError:
            out.append({'need': '?', 'reason': f'ligne illisible : {line[:120]}', 'test': '?', 'binary': '?'})
    return out


def read_list(path: Path) -> list[tuple[str, str, str]]:
    out = []
    for n, line in enumerate(path.read_text(encoding='utf-8').splitlines(), 1):
        line = line.strip()
        if not line or line.startswith('#'):
            continue
        parts = line.split()
        if len(parts) != 3:
            raise ValueError(f'{path}:{n} : trois champs attendus (journal, binaire, test), lu « {line} »')
        out.append((parts[0], parts[1], parts[2]))
    return out


def probe_value(path: Path | None):
    """Le champ `available` de la ligne « gpu_probe: {…} », et l'adaptateur, ou (None, None)."""
    if path is None or not path.exists():
        return None, None
    for raw in path.read_text(encoding='utf-8', errors='replace').splitlines():
        line = ANSI.sub('', raw)
        i = line.find('gpu_probe: ')
        if i < 0:
            continue
        try:
            data = json.loads(line[i + len('gpu_probe: '):])
        except json.JSONDecodeError:
            return None, None
        pipe = data.get('pipeline', {})
        return pipe.get('available'), pipe.get('adapter')
    return None, None


def judge(expected, logs, skips, gpu_logs, probe, probe_off):
    """Liste des défauts (étiquette, détail) et état de chaque attendu."""
    faults, states = [], {}
    parsed = {name: read_log(path) for name, path in logs.items()}
    for (log, binary, test) in expected:
        if log not in parsed:
            faults.append(('journal-absent', f'{log} : aucun journal pour {binary} {test}'))
            states[(log, binary, test)] = 'journal absent'
            continue
        results, counts = parsed[log]
        if counts.get(binary, 0) == 0:
            faults.append(('binaire-muet', f'{log} {binary} : aucune ligne de résultat'))
        got = results.get((binary, test), [])
        if not got:
            state = 'absent'
            faults.append(('absent', f'{log} {binary} {test} : n\'a pas tourné'))
        elif len(got) > 1:
            state = 'doublon'
            faults.append(('doublon', f'{log} {binary} {test} : {len(got)} résultats'))
        elif got[0] == 'ignored':
            state = 'ignoré'
            faults.append(('ignore', f'{log} {binary} {test} : ignoré'))
        elif got[0] == 'FAILED':
            state = 'en échec'
            faults.append(('echec', f'{log} {binary} {test} : en échec'))
        else:
            state = 'vert'
        for entry in skips.get(log, []):
            if entry.get('binary') == binary and entry.get('test') == test:
                state = 'sauté'
                faults.append(('saute', f'{log} {binary} {test} : sauté ({entry.get("need")}, {entry.get("reason")})'))
        states[(log, binary, test)] = state
    for name, (results, _) in parsed.items():
        for (binary, test), got in results.items():
            if 'FAILED' in got and (name, binary, test) not in states:
                faults.append(('echec', f'{name} {binary} {test} : en échec (hors liste)'))
    for name in gpu_logs:
        for entry in skips.get(name, []):
            if entry.get('need') == 'gpu':
                faults.append(('saut-gpu', f'{name} : saut GPU de {entry.get("binary")} {entry.get("test")} ({entry.get("reason")})'))
    if probe is not None:
        available, _ = probe_value(probe)
        if available is not True:
            faults.append(('sonde', f'sonde du noyau graphique : available = {available!r}, true attendu'))
    if probe_off is not None:
        available, _ = probe_value(probe_off)
        if available is not False:
            faults.append(('temoin-sonde', f'témoin de la sonde (sans adaptateur logiciel admis) : available = {available!r}, false attendu'))
    return faults, states, parsed


def summary_text(expected, states, parsed, faults, probe) -> str:
    lines = ['### Porte tests Rust (schéma des réglages, pipeline, accord des deux chemins sur lavapipe)', '']
    _, adapter = probe_value(probe)
    lines.append(f'Adaptateur de la sonde : {adapter or "aucun"}.')
    lines += ['', '| Journal | Binaire | Lignes de résultat | Verts | Ignorés | En échec |', '|---|---|---|---|---|---|']
    for name, (results, counts) in sorted(parsed.items()):
        for binary in sorted(counts):
            states_b = [s for (b, _), got in results.items() if b == binary for s in got]
            lines.append(f'| {name} | {binary} | {counts[binary]} | {states_b.count("ok")} | {states_b.count("ignored")} | {states_b.count("FAILED")} |')
    lines += ['', '| Attendu | État |', '|---|---|']
    for key in expected:
        lines.append(f'| {" ".join(key)} | {states.get(key, "?")} |')
    lines += ['', f'Verdict : {"ROUGE" if faults else "VERT"} ({len(faults)} défaut(s)).']
    for label, why in faults:
        lines.append(f'- [{label}] {why}')
    return '\n'.join(lines) + '\n'


def pairs(values):
    out = {}
    for v in values or []:
        name, sep, path = v.partition('=')
        if not sep or not name or not path:
            raise ValueError(f'« {v} » : NOM=CHEMIN attendu')
        out[name] = Path(path)
    return out


def parse_args(argv):
    opts = {'log': [], 'skips': [], 'gpu-log': []}
    i = 0
    while i < len(argv):
        key = argv[i]
        if not key.startswith('--') or i + 1 >= len(argv):
            raise ValueError(f'argument inattendu : {key}')
        name, value = key[2:], argv[i + 1]
        if name in opts and isinstance(opts[name], list):
            opts[name].append(value)
        else:
            opts[name] = value
        i += 2
    return opts


def check(argv) -> int:
    opts = parse_args(argv)
    expected = read_list(Path(opts['list']))
    logs = pairs(opts['log'])
    skips = {name: read_skips(path) for name, path in pairs(opts['skips']).items()}
    probe = Path(opts['probe']) if 'probe' in opts else None
    probe_off = Path(opts['probe-off']) if 'probe-off' in opts else None
    faults, states, parsed = judge(expected, logs, skips, opts['gpu-log'], probe, probe_off)
    text = summary_text(expected, states, parsed, faults, probe)
    sys.stdout.write(text)
    if 'summary' in opts:
        with open(opts['summary'], 'a', encoding='utf-8') as f:
            f.write(text)
    for label, why in faults:
        sys.stdout.write(f'::error::porte tests Rust [{label}] {why}\n')
    return 1 if faults else 0


# --- auto-test : journaux fabriqués, chaque défaut doit rougir avec son étiquette

HEAD = '     Running unittests src/lib.rs (target/debug/deps/evolight_pipeline-0123456789abcdef)\n\nrunning 3 tests\n'
GPU = '     Running tests/it/main.rs (/home/runner/.evora-target-tests/debug/deps/it-fedcba9876543210)\n\nrunning 1 test\n'
LIST = 'cpu evolight_pipeline tests_evora1::a\ncpu evolight_pipeline tests_evora1::b\n# commentaire\n\ngpu it equivalence::c\n'
PROBE_ON = 'test gpu_probe::gpu_probe ... \ngpu_probe: {"pipeline":{"available":true,"adapter":"llvmpipe (LLVM 19)","reason":null}}\nok\n'
PROBE_OFF = 'gpu_probe: {"pipeline":{"available":false,"adapter":null,"reason":"software adapter refused"}}\n'


def run_case(tmp: Path, name: str, cpu: str, gpu: str, cpu_skips='', gpu_skips='', probe=PROBE_ON, probe_off=PROBE_OFF):
    d = tmp / name
    d.mkdir()
    for f, text in (('list', LIST), ('cpu', cpu), ('gpu', gpu), ('scpu', cpu_skips), ('sgpu', gpu_skips),
                    ('probe', probe), ('off', probe_off)):
        (d / f).write_text(text, encoding='utf-8')
    expected = read_list(d / 'list')
    skips = {'cpu': read_skips(d / 'scpu'), 'gpu': read_skips(d / 'sgpu')}
    faults, _, _ = judge(expected, {'cpu': d / 'cpu', 'gpu': d / 'gpu'}, skips, ['gpu'], d / 'probe', d / 'off')
    return {label for label, _ in faults}


def selftest() -> int:
    good_cpu = HEAD + 'test tests_evora1::a ... ok\ntest tests_evora1::b ... ok\ntest tests_frozen::p ... ignored, P0\n\ntest result: ok.\n'
    good_gpu = GPU + 'test equivalence::c ... ok\n\ntest result: ok.\n'
    colour = '\x1b[1m\x1b[92m' + HEAD.replace('Running', 'Running\x1b[0m') + 'test tests_evora1::a ... \x1b[32mok\x1b[0m\ntest tests_evora1::b ... ok\n'
    cases = [
        ('conforme', dict(cpu=good_cpu, gpu=good_gpu), set()),
        ('couleurs', dict(cpu=colour, gpu=good_gpu), set()),
        ('absent', dict(cpu=good_cpu.replace('test tests_evora1::b ... ok\n', ''), gpu=good_gpu), {'absent'}),
        ('ignore', dict(cpu=good_cpu.replace('tests_evora1::b ... ok', 'tests_evora1::b ... ignored'), gpu=good_gpu), {'ignore'}),
        ('echec', dict(cpu=good_cpu.replace('tests_evora1::b ... ok', 'tests_evora1::b ... FAILED'), gpu=good_gpu), {'echec'}),
        ('doublon', dict(cpu=good_cpu + 'test tests_evora1::a ... ok\n', gpu=good_gpu), {'doublon'}),
        ('echec-hors-liste', dict(cpu=good_cpu + 'test tests::autre ... FAILED\n', gpu=good_gpu), {'echec'}),
        ('saut-attendu', dict(cpu=good_cpu, gpu=good_gpu,
                              cpu_skips='{"need":"corpus","reason":"x","test":"tests_evora1::a","binary":"evolight_pipeline","location":"a.rs:1"}\n'), {'saute'}),
        ('saut-gpu', dict(cpu=good_cpu, gpu=good_gpu,
                          gpu_skips='{"need":"gpu","reason":"no GPU adapter","test":"equivalence::z","binary":"it","location":"e.rs:9"}\n'), {'saut-gpu'}),
        ('binaire-muet', dict(cpu=good_cpu, gpu=GPU), {'binaire-muet', 'absent'}),
        ('autre-binaire', dict(cpu=good_cpu.replace('evolight_pipeline-0123456789abcdef', 'evolight_develop-0123456789abcdef'), gpu=good_gpu),
         {'binaire-muet', 'absent'}),
        ('sonde-false', dict(cpu=good_cpu, gpu=good_gpu, probe=PROBE_OFF), {'sonde'}),
        ('temoin-true', dict(cpu=good_cpu, gpu=good_gpu, probe_off=PROBE_ON), {'temoin-sonde'}),
        ('sonde-absente', dict(cpu=good_cpu, gpu=good_gpu, probe='rien\n'), {'sonde'}),
    ]
    failures = 0
    with tempfile.TemporaryDirectory(prefix='evora-test-guards-') as t:
        for name, kw, want in cases:
            got = run_case(Path(t), name, **kw)
            ok = got == want
            failures += 0 if ok else 1
            sys.stdout.write(f'{"ok " if ok else "KO "} {name} : attendu {sorted(want) or "vert"}, obtenu {sorted(got) or "vert"}\n')
    sys.stdout.write(f'auto-test de la porte tests Rust : {"VERT" if failures == 0 else "ROUGE"} ({len(cases)} cas)\n')
    return 1 if failures else 0


def main(argv) -> int:
    if not argv:
        sys.stderr.write(__doc__)
        return 2
    try:
        if argv[0] == 'selftest':
            return selftest()
        if argv[0] == 'check':
            return check(argv[1:])
    except (ValueError, KeyError, OSError) as e:
        sys.stderr.write(f'test-guards.py : {e}\n')
        return 2
    sys.stderr.write(__doc__)
    return 2


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
