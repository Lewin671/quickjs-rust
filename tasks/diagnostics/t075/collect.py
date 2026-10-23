"""T075 development-only address sampler; run inside the serialized iMac queue.

Strip symbol tables from copies so sample cannot merge distinct PCs by symbol.
Keep original images for symbolization; assert identical executable sections.
No runtime instrumentation or workload amplification. This is time sampling.
"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess

from tools.benchmark.screen import resolve_cases, DEFAULT_CACHE


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def sections(path):
    data = Path(path).read_bytes()
    assert struct.unpack_from('<I', data)[0] == 0xfeedfacf
    result = {}
    offset = 32
    for _ in range(struct.unpack_from('<I', data, 16)[0]):
        command, size = struct.unpack_from('<II', data, offset)
        if command == 0x19:
            for i in range(struct.unpack_from('<I', data, offset + 64)[0]):
                at = offset + 72 + 80 * i
                name, segment, address, length, file_offset = struct.unpack_from('<16s16sQQI', data, at)
                name, segment = name.rstrip(b'\0').decode(), segment.rstrip(b'\0').decode()
                if segment == '__TEXT':
                    raw = data[file_offset:file_offset + length]
                    result[name] = {'address': address, 'bytes': length, 'sha256': hashlib.sha256(raw).hexdigest()}
        offset += size
    return result


def main():
    out = Path(os.environ['BENCH_OUT'])
    build = Path(os.environ['BENCH_QUEUE']) / '20261023T183253Z-t075-build-and-pilot/out'
    canonical = Path((build / 'canonical-path.txt').read_text().strip())
    images = {'canonical': canonical, 'debug': Path(os.environ['BENCH_QUEUE']) / '20261023T183753Z-t075-metadata-preserving-debug/out/debug-qjs',
              'ng': Path('third_party/quickjs-ng/build/qjs').resolve()}
    identity = {'revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
                'images': {}, 'clock': subprocess.check_output(['date'], text=True).strip()}
    for role, binary in images.items():
        target = out / (role + '-stripped')
        shutil.copy2(binary, target)
        subprocess.run(['strip', target], check=True)
        # CoreSymbolication otherwise finds the original by UUID and merges PCs.
        data = bytearray(target.read_bytes())
        offset = 32
        for _ in range(struct.unpack_from('<I', data, 16)[0]):
            command, size = struct.unpack_from('<II', data, offset)
            if command == 0x1b:
                data[offset + 8:offset + 24] = hashlib.sha256(data).digest()[:16]
            offset += size
        target.write_bytes(data)
        before, after = sections(binary), sections(target)
        assert before == after, (role, 'strip altered text sections')
        identity['images'][role] = {'path': str(binary), 'sha256': digest(binary), 'sections': before,
                                   'sampled_sha256': digest(target)}
        (out / (role + '-nm.txt')).write_bytes(subprocess.check_output(['nm', '-n', binary]))
    (out / 'identity.json').write_text(json.dumps(identity, indent=2) + '\n')
    (out / 'workloads').mkdir()
    cases = resolve_cases(['development'], DEFAULT_CACHE, out / 'workloads')
    assert len(cases) == 10 and all(c.id.startswith('development/') for c in cases)
    rows = []
    for case in cases:
        for repeat in range(3):
            roles = ['canonical', 'debug', 'ng'] if repeat % 2 == 0 else ['ng', 'debug', 'canonical']
            for role in roles:
                folder = out / 'profiles' / case.id.replace('/', '--') / role / str(repeat)
                folder.mkdir(parents=True)
                argv = [str(out / (role + '-stripped')), '--script' if role == 'ng' else '--raw', str(case.workload)]
                with (folder / 'stdout').open('w') as stdout, (folder / 'stderr').open('w') as stderr:
                    process = subprocess.Popen(argv, stdout=stdout, stderr=stderr)
                    command = ['sample', str(process.pid), '60', '1', '-mayDie', '-nodsyms', '-file', str(folder / 'raw.sample')]
                    sampler = subprocess.run(command, capture_output=True, text=True, timeout=100)
                    ended = process.poll() is not None
                    status = process.wait(timeout=180)
                (folder / 'sampler.log').write_text(sampler.stdout + sampler.stderr)
                expected = case.expected_stdout
                if role == 'ng':
                    expected = expected.removesuffix('__QJS_EXTERNAL_OK__\n')
                assert status == 0 and sampler.returncode == 0, (role, case.id, sampler.stderr)
                assert (folder / 'stdout').read_text() == expected, (role, case.id, 'checksum')
                raw = folder / 'raw.sample'
                assert 'Call graph:' in raw.read_text() and ended
                rows.append({'id': case.id, 'repeat': repeat, 'role': role, 'pid': process.pid,
                             'argv': argv, 'sample_command': command, 'process_exit': status,
                             'sampler_exit': sampler.returncode, 'ended_at_sampler_return': ended,
                             'raw_path': str(raw.relative_to(out)), 'raw_sha256': digest(raw),
                             'workload_sha256': digest(case.workload), 'stdout_sha256': digest(folder / 'stdout')})
                (out / 'profiles.json').write_text(json.dumps(rows, indent=2) + '\n')
                print('PROFILE', case.id, repeat, role, flush=True)
    print('COMPLETE', len(rows), flush=True)


if __name__ == '__main__':
    main()
