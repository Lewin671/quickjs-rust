"""One exact-image root-cause pass; sample self counts and static disassembly."""
from pathlib import Path
from collections import Counter
import argparse, dataclasses, hashlib, json, os, re, subprocess, sys
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from tools.benchmark.screen import counter_tool, sample
from tools.benchmark.layout_symbols import parse_symbol
from tools.benchmark.layout_tables import text_symbols

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--triage', type=Path, required=True)
p.add_argument('--output', type=Path, required=True)
p.add_argument('--repeats', type=int, default=8)
p.add_argument('--case', action='append', default=[], help='quality supplement restricted to the frozen top five')
a = p.parse_args()
a.output.mkdir(parents=True, exist_ok=False)
os.environ['TZ'] = 'UTC'
for key in ['QJS_DISABLE_IC', 'QJS_VERIFY_IC', 'QJS_TL_TRACE']:
 os.environ.pop(key, None)
report = json.loads((a.triage / 'triage.json').read_text())
cases = sorted((r for r in report['cases'] if r['id'].startswith(('external/', 'broad/'))), key=lambda r: r['wall_ns']['ratio'], reverse=True)[:5]
assert len(cases) == 5
if a.case:
 assert set(a.case).issubset({c['id'] for c in cases})
 cases = [c for c in cases if c['id'] in a.case]
binaries = {role: Path(report['builds'][role]['path']) for role in ['base', 'candidate']}
for role, binary in binaries.items():
 assert hashlib.sha256(binary.read_bytes()).hexdigest() == report['builds'][role]['sha256']


def canonical(raw):
 parsed = parse_symbol(raw)
 return parsed.canonical if parsed else raw


def parse_self(text):
 marker = 'Sort by top of stack, same collapsed (when >= 5):'
 assert marker in text, 'sample has no self table'
 body = text.split(marker, 1)[1].split('Binary Images:', 1)[0]
 result = Counter()
 for line in body.splitlines():
  match = re.match(r'^\s*(\S.*?)\s+\(in qjs\)\s+(\d+)\s*$', line)
  if match:
   name = match[1].split()[0]
   raw = '_' + name if name.startswith('_R') else name
   result[raw] += int(match[2])
 return result


static = {}
for role, binary in binaries.items():
 symbols = text_symbols(binary)
 counts = Counter()
 process = subprocess.Popen(['otool', '-tvV', str(binary)], stdout=subprocess.PIPE, text=True)
 current = None
 for line in process.stdout:
  if line.rstrip().endswith(':') and not line[0].isspace(): current = line.strip()[:-1]
  elif current and re.match(r'^[0-9a-fA-F]{16}\s+[a-z]', line): counts[current] += 1
 assert process.wait() == 0
 static[role] = {raw: {'canonical': canonical(raw), 'decoded_instructions': count, 'extent_bytes': symbols.size_of(raw), 'address': symbols.address_of(raw)} for raw, count in counts.items()}
(a.output / 'static-functions.json').write_text(json.dumps(static, indent=2))
tool = counter_tool()
results = []
for case in cases:
 case_id = case['id']; folder = a.output / case_id.replace('/', '--'); folder.mkdir()
 packet = a.triage / 'variant0' / (case_id.replace('/', '--') + '.jsonl')
 rows = [json.loads(line) for line in packet.open()]
 argv = {role: next(r['argv'] for r in reversed(rows) if r['argv'][0] == str(binary)) for role, binary in binaries.items()}
 assert argv['base'][1:] == argv['candidate'][1:]
 source = next(Path(arg) for arg in argv['base'][1:] if arg.endswith('.js'))
 record = {'case': case_id, 'triage_wall_ratio': case['wall_ns']['ratio'], 'triage_instructions_ratio': case['instructions']['ratio'], 'workload_sha256': hashlib.sha256(source.read_bytes()).hexdigest(), 'argv': argv, 'roles': {}}
 expected = None
 for role in binaries:
  counters = [dataclasses.asdict(sample(tool, argv[role], 120.0)) for _ in range(3)]
  for item in counters:
   if expected is None: expected = item['stdout']
   assert item['stdout'] == expected
  self_counts = Counter(); samples = []
  for rep in range(a.repeats):
   prefix = folder / f'{role}-{rep}'
   with prefix.with_suffix('.stdout').open('w') as stdout, prefix.with_suffix('.stderr').open('w') as stderr:
    proc = subprocess.Popen(argv[role], stdout=stdout, stderr=stderr, env={**os.environ, 'TZ': 'UTC'})
    path = prefix.with_suffix('.sample')
    sampler = subprocess.run(['/usr/bin/sample', str(proc.pid), '1', '1', '-mayDie', '-file', str(path)], capture_output=True, text=True)
    code = proc.wait(timeout=120)
   assert code == 0 and prefix.with_suffix('.stdout').read_text() == expected
   prefix.with_suffix('.sample-log').write_text(sampler.stdout + sampler.stderr)
   assert path.exists() and path.stat().st_size
   counts = parse_self(path.read_text(errors='replace')); assert counts, (case_id, role, rep)
   self_counts.update(counts)
   samples.append({'path': str(path.relative_to(a.output)), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()})
  functions = [{'raw': raw, 'canonical': canonical(raw), 'self_samples': count, 'approx_self_ms': count, **{k: v for k, v in static[role].get(raw, {}).items() if k != 'canonical'}} for raw, count in self_counts.most_common()]
  record['roles'][role] = {'counters': counters, 'sample_files': samples, 'self_table_samples': sum(self_counts.values()), 'functions': functions}
 results.append(record)
 (a.output / 'profiles.json').write_text(json.dumps({'source': report['builds']['candidate']['revision'], 'images': report['builds'], 'cycle_counter_qualified': tool.cycles, 'repeats': a.repeats, 'method': 'repeated exact original argv per image/case; sample 1s at 1ms mayDie; self tables omit functions with fewer than five samples per report; static decoded instructions are not per-function dynamic retired instructions; separate unprofiled whole-case counters', 'records': results}, indent=2))
 print('PROFILED', case_id, {r: record['roles'][r]['self_table_samples'] for r in binaries}, flush=True)
