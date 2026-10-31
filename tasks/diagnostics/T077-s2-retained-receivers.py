"""Repeat the original retained-receiver inputs; resource counts, no timing claim."""
from pathlib import Path
import argparse, hashlib, json, re, subprocess
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--base', type=Path, required=True)
p.add_argument('--candidate', type=Path, required=True)
p.add_argument('--revision', required=True)
p.add_argument('--output', type=Path, required=True)
p.add_argument('--pairs', type=int, default=3)
a = p.parse_args()
assert a.pairs > 0
binaries = {'base': a.base.resolve(), 'candidate': a.candidate.resolve()}
a.output.mkdir(parents=True, exist_ok=False)
# Names and direct calls match the original inputs exactly. "nocall" is the
# historical label for omitting the method call; fn_mem_nocall still calls f().
scripts = {
 'arr_mem': 'var keep = [];\nfor (var i = 0; i < 300000; i++) { var a = [i, i]; a.indexOf(i); keep[i] = a; }\nprint(keep.length);\n',
 'arr_mem_nocall': 'var keep = [];\nfor (var i = 0; i < 300000; i++) { var a = [i, i]; keep[i] = a; }\nprint(keep.length);\n',
 'fn_mem': 'var keep = [];\nfor (var i = 0; i < 300000; i++) { var f = function () { return 1; }; f.call(null); keep[i] = f; }\nprint(keep.length);\n',
 'fn_mem_nocall': 'var keep = [];\nfor (var i = 0; i < 300000; i++) { var f = function () { return 1; }; f(); keep[i] = f; }\nprint(keep.length);\n',
}
rows = []
for name, source in scripts.items():
 path = a.output / (name + '.js')
 path.write_text(source)
 expected = None
 for repeat in range(a.pairs):
  for role, binary in binaries.items():
   result = subprocess.run(['/usr/bin/time', '-l', str(binary), str(path.resolve())], capture_output=True, text=True, timeout=180)
   assert result.returncode == 0, (name, role, result.stderr)
   if expected is None: expected = result.stdout
   assert result.stdout == expected
   (a.output / f'{role}-{name}-{repeat}.rusage').write_text(result.stderr)
   row = {'case': name, 'role': role, 'repeat': repeat,
          'peak_rss_bytes': int(re.search(r'(\d+)\s+maximum resident set size', result.stderr)[1]),
          'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
          'source_sha256': hashlib.sha256(source.encode()).hexdigest()}
   rows.append(row)
   print('RSS', json.dumps(row), flush=True)
(a.output / 'rss.json').write_text(json.dumps({'revision': a.revision, 'base': 'dcee3c56',
 'note': 'diagnostic resource counts only; no timing or formal acceptance claim', 'records': rows}, indent=2) + '\n')
