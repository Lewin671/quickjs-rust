// Held-out driver: d3-array (ISC) summarizes and bins generated numeric data.
// Deterministic. Prints one line.
var N = 200000;

var data = [];
var seed = 12345;
for (var i = 0; i < N; i++) {
  seed = (seed * 1103515245 + 12345) & 0x7fffffff;
  data.push((seed % 100000) / 1000);
}

var hash = 0;
function mix(value) {
  hash = ((hash << 5) - hash + value) | 0;
}
function mixFloat(value) {
  mix(Math.round(value * 1000));
}

var extent = d3.extent(data);
mixFloat(extent[0]);
mixFloat(extent[1]);
mixFloat(d3.mean(data));
mixFloat(d3.deviation(data));
mixFloat(d3.median(data));
mixFloat(d3.quantile(data, 0.25));
mixFloat(d3.quantile(data, 0.75));

var bins = d3.bin().thresholds(50)(data);
for (var b = 0; b < bins.length; b++) {
  mix(bins[b].length);
  mixFloat(bins[b].x0);
}

var grouped = d3.group(data, function (value) { return Math.round(value) % 10; });
grouped.forEach(function (values, key) { mix(key); mix(values.length); });

var sorted = d3.sort(data.slice(0, 5000), function (a, b) { return a - b; });
mixFloat(sorted[0]);
mixFloat(sorted[sorted.length - 1]);
print("CHECKSUM " + (hash >>> 0).toString(16));
