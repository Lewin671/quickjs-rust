// Derived from: test/built-ins/Number/parseInt/not-a-constructor.js
if (Number.parseInt("10", 2) !== 2) {
  throw "expected Number.parseInt to parse with radix";
}
if (Number.parseFloat("3.5px") !== 3.5) {
  throw "expected Number.parseFloat to parse decimal prefix";
}
if (Number.parseInt.length !== 2) {
  throw "expected Number.parseInt.length to be 2";
}
if (Number.parseFloat.length !== 1) {
  throw "expected Number.parseFloat.length to be 1";
}
