// Derived from: test/built-ins/Math/random/S15.8.2.14_A1.js
for (var i = 0; i < 20; i++) {
  var value = Math.random();
  if (typeof value !== "number" || value < 0 || value >= 1) {
    throw "Math.random should return a number in [0, 1)";
  }
}
