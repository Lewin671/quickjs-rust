(function() {
  var values = [
    typeof 1n,
    String(1n + 2n),
    String(7n / 2n),
    String(BigInt.asIntN(2, 3n)),
    String(BigInt.asUintN(2, -1n)),
    String((10n).toString(16)),
    String(1n == 1),
    String(1n === 1),
    Object.prototype.toString.call(1n)
  ];
  return values.join(":");
})()
