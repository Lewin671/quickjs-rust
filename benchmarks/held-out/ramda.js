// Held-out driver: ramda (MIT) transforms generated arrays through functional pipelines.
// Deterministic. Prints one line.
var N = 14000;

var hash = 0;
function mix(value) {
  hash = ((hash << 5) - hash + value) | 0;
}
function row(i) {
  return { id: i, bucket: i % 12, value: (i * 7) % 100 };
}
function summarize(rows) {
  var mapped = R.map(function (r) { return r.value; }, rows);
  var filtered = R.filter(function (v) { return v % 3 !== 0; }, mapped);
  var total = R.reduce(function (acc, v) { return acc + v; }, 0, filtered);
  var byBucket = R.groupBy(function (r) { return String(r.bucket % 5); }, rows);
  var sorted = R.sortBy(function (r) { return -r.value; }, rows);
  return total + R.keys(byBucket).length * 1000 + sorted[0].value;
}

for (var iteration = 0; iteration < N; iteration++) {
  var rows = [];
  for (var i = 0; i < 40; i++) rows.push(row(iteration * 40 + i));
  mix(summarize(rows));
  mix(R.take(3, R.reverse(rows))[0].id);
}
print("CHECKSUM " + (hash >>> 0).toString(16));
