// Derived from: test/built-ins/Number/prototype/toPrecision/return-values.js
// Derived from: test/built-ins/Number/prototype/toPrecision/length.js
// Derived from: test/built-ins/Number/prototype/toPrecision/range.js
if (Number.prototype.toPrecision.length !== 1) {
  throw "expected Number.prototype.toPrecision.length to be 1";
}
if (Number.prototype.toPrecision() !== "0") {
  throw "expected Number.prototype.toPrecision() to return 0";
}
if ((123.456).toPrecision(5) !== "123.46") {
  throw "expected toPrecision to round significant digits";
}
if ((123.456).toPrecision(2) !== "1.2e+2") {
  throw "expected toPrecision to use exponential form when exponent >= precision";
}
if ((0.0001234).toPrecision(5) !== "0.00012340") {
  throw "expected toPrecision to pad significant digits";
}
if (NaN.toPrecision(101) !== "NaN") {
  throw "expected NaN toPrecision to ignore precision range";
}
var caught = false;
try {
  (3).toPrecision(0);
} catch (error) {
  caught = error instanceof RangeError;
}
if (!caught) {
  throw "expected toPrecision to throw RangeError for precision < 1";
}
