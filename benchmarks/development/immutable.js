// Development driver: immutable.js (MIT) builds persistent collections.
// Deterministic: every value is a pure function of N. Prints one line.
var N = 25000;

var hash = 0;
function mix(value) {
  hash = ((hash << 5) - hash + value) | 0;
}

var list = Immutable.List();
for (var i = 0; i < N; i++) list = list.push({ id: i, bucket: i % 17, value: (i * 31) % 251 });

var mapped = list.map(function (row) { return row.value; });
var filtered = list.filter(function (row) { return row.value % 3 !== 0; });
var total = list.reduce(function (acc, row) { return acc + row.value; }, 0);
var sorted = list.sort(function (left, right) { return right.value - left.value; });
var grouped = list.groupBy(function (row) { return String(row.bucket % 5); });

mix(list.size);
mix(mapped.size);
mix(filtered.size);
mix(total);
mix(sorted.first().value + sorted.last().value);
mix(grouped.size);
mix(grouped.get("0").size);

var map = Immutable.Map();
for (var i = 0; i < N; i++) map = map.set("k" + (i % 97), i);
var nested = map.setIn(["a", "b", "c"], 7);
mix(map.size);
mix(nested.getIn(["a", "b", "c"]));
mix(map.get("k0") + map.get("k96"));

var updated = list.withMutations(function (draft) {
  for (var i = 0; i < 200; i++) draft.set(i, { id: i, bucket: 0, value: i });
});
mix(updated.get(199).value);
mix(updated.get(500).value);

var set = Immutable.Set(list.map(function (row) { return row.bucket; }));
mix(set.size);
mix(set.has(3) ? 1 : 0);
mix(Immutable.is(list.toList(), list.toList()) ? 1 : 0);
print("CHECKSUM " + (hash >>> 0).toString(16));
