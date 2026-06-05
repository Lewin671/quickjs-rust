// Derived from: test/built-ins/Number/prototype/toFixed/S15.7.4.5_A1.1_T01.js
// Derived from: test/built-ins/Number/prototype/toFixed/range.js
// Derived from: test/built-ins/Number/prototype/toFixed/length.js
if (Number.prototype.toFixed.length !== 1) {
  throw "expected Number.prototype.toFixed.length to be 1";
}
if (Number.prototype.toFixed() !== "0") {
  throw "expected Number.prototype.toFixed() to return 0";
}
if (Number.prototype.toFixed(1) !== "0.0") {
  throw "expected Number.prototype.toFixed(1) to return 0.0";
}
if ((3).toFixed(2) !== "3.00") {
  throw "expected toFixed to pad fractional digits";
}
if ((123.456).toFixed(1) !== "123.5") {
  throw "expected toFixed to round fractional digits";
}
if ((1e21).toFixed(2) !== "1e+21") {
  throw "expected toFixed to use Number string form for numbers >= 1e21";
}
var caught = false;
try {
  (3).toFixed(101);
} catch (error) {
  caught = error instanceof RangeError;
}
if (!caught) {
  throw "expected fraction digits greater than 100 to throw RangeError";
}
