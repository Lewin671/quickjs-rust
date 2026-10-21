"""Source-bound event attribution in an instrumented isolated archive, no timing."""
import os,subprocess,sys
from pathlib import Path
OUT=Path(os.environ['BENCH_OUT']);assert 'no-timing' in (OUT.parent/'status').read_text()
# Reuse the original generic instrumentation producer with two additional cases
# and counters. Runtime in the runner is never edited.
source=Path('tasks/diagnostics/t056-pragmatic/events.py').read_text()
source=source.replace("target/t056-pragmatic-events", "target/t056-hot-cold-events")
source=source.replace("diag_layout_key_compares,", "diag_layout_key_compares,\\n    diag_atom_reused_text,\\n    diag_zero_index_class_segments,")
needle="p=source/'crates/qjs-runtime/src/builtins.rs'"
injection='''p=source/'crates/qjs-runtime/src/property/atom.rs';s=p.read_text();s=s.replace('Rc::clone(text)\\n', 'crate::diagnostics::count!(diag_atom_reused_text);\\n                Rc::clone(text)\\n',1);p.write_text(s)
p=source/'crates/qjs-runtime/src/value/object/class_fields.rs';s=p.read_text();s=s.replace('self.0.add_index_properties(added_indices);','if added_indices == 0 { crate::diagnostics::count!(diag_zero_index_class_segments); }\\n        self.0.add_index_properties(added_indices);');p.write_text(s)
'''
assert needle in source;source=source.replace(needle,injection+needle,1)
source=source.replace("case=resolve_cases(['external/sunspider-1.0/string-unpack-code'],DEFAULT_CACHE,work)[0]", "cases=resolve_cases(['external/sunspider-1.0/string-unpack-code','external/jetstream3-js-subset/raytrace-public-class-fields'],DEFAULT_CACHE,work)")
source=source.replace("'unpack':['--raw',str(case.workload)]", "'unpack':['--raw',str(cases[0].workload)],'class_fields':['--raw',str(cases[1].workload)]")
producer=OUT/'events-producer.py';producer.write_text(source)
subprocess.run([sys.executable,str(producer)],check=True)
