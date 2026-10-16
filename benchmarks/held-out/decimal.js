// Held-out driver: decimal.js (MIT) runs arbitrary-precision arithmetic loops.
// Only +,-,*,/ and sqrt, so results are bit-identical across engines.
// Deterministic. Prints one line.
Decimal.set({ precision: 40 });

var N = 8000;
var hash = 0;
function mix(value) {
  hash = ((hash << 5) - hash + value) | 0;
}

var base = new Decimal("1.0000000000000001");
for (var iteration = 0; iteration < N; iteration++) {
  var k = new Decimal(iteration % 17 + 1);
  var accrued = base
    .times(k)
    .plus(new Decimal("0.000000000000003"))
    .dividedBy(new Decimal(iteration % 13 + 2))
    .minus(base.dividedBy(k));
  var rooted = accrued.abs().plus(2).sqrt();
  var text = rooted.toFixed(20);
  mix(text.length);
  mix(text.charCodeAt(iteration % 20));
}
print("CHECKSUM " + (hash >>> 0).toString(16));
