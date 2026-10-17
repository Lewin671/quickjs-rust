// Development driver: js-yaml (MIT) loads, dumps and reloads a generated document.
// Deterministic: the document is a pure function of N. Prints one line.
var N = 1400;

var parts = ["meta:\n  version: 4.1.0\n  flags: [a, b, c]\nrecords:\n"];
for (var i = 0; i < N; i++) {
  parts.push("  - id: " + i + "\n");
  parts.push("    name: \"record " + (i % 29) + "\"\n");
  parts.push("    ratio: " + ((i % 97) / 8) + "\n");
  parts.push("    enabled: " + (i % 2 === 0 ? "true" : "false") + "\n");
  parts.push("    tags: [" + (i % 5) + ", " + ((i * 3) % 11) + "]\n");
  parts.push("    note: |\n      multi line " + i + "\n      second " + i + "\n");
  parts.push("    null_value: null\n");
  parts.push("    nested:\n      deep: { a: " + i + ", b: 'x" + (i % 7) + "' }\n");
}
var document = parts.join("");

var loaded = jsyaml.load(document);
var dumped = jsyaml.dump(loaded, { lineWidth: 120, noRefs: true });
var reloaded = jsyaml.load(dumped);
var json = jsyaml.load('{"a": [1, 2, {"b": true}], "c": null}');

var hash = 0;
function mix(value) {
  hash = ((hash << 5) - hash + value) | 0;
}
mix(loaded.records.length);
mix(loaded.meta.flags.length);
mix(dumped.length);
for (var i = 0; i < dumped.length; i += 53) mix(dumped.charCodeAt(i));
mix(reloaded.records.length);
for (var i = 0; i < reloaded.records.length; i += 41) {
  var record = reloaded.records[i];
  mix(record.id);
  mix(record.name.length);
  mix(Math.round(record.ratio * 8));
  mix(record.enabled ? 1 : 0);
  mix(record.tags[0] + record.tags[1]);
  mix(record.note.length);
  mix(record.nested.deep.a + record.nested.deep.b.length);
}
mix(json.a[2].b ? 1 : 0);
mix(json.a[0] + json.a[1]);
mix(json.c === null ? 1 : 0);
mix(jsyaml.dump({ a: 1, b: [2, 3], c: { d: true } }, { flowLevel: 0 }).length);
print("CHECKSUM " + (hash >>> 0).toString(16));
