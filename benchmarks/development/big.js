// Development driver: big.js (MIT) runs arbitrary-precision decimal loops.
// Deterministic: only +,-,*,/ and sqrt, so digits match across engines. One line.
Big.DP = 40;
Big.RM = 1;

var N = 300;

var hash = 0;
function mix(value) {
  hash = ((hash << 5) - hash + value) | 0;
}

var base = new Big("1.0000000000000001");
var accumulated = new Big(0);
for (var i = 0; i < N; i++) {
  var k = new Big((i % 17) + 1);
  var term = base
    .times(k)
    .plus("0.000000000000003")
    .div(new Big((i % 13) + 2))
    .minus(base.div(k));
  accumulated = accumulated.plus(term);
  var rooted = accumulated.abs().plus(2).sqrt();
  var text = rooted.toString();
  var fixed = rooted.toFixed(20);
  mix(text.length);
  mix(text.charCodeAt(i % text.length));
  mix(fixed.length);
  mix(Number(rooted.round(8).toString().split(".")[1].charAt(0)));
  mix(term.cmp(0) + 1);
  mix(rooted.pow(2).toPrecision(12).length);
  mix(accumulated.mod(7).toFixed(0).length);
  mix(rooted.gt("1") ? 1 : 0);
}
print("CHECKSUM " + (hash >>> 0).toString(16));
