// Derived from: test/built-ins/Number/prototype/toExponential/return-values.js
// Derived from: test/built-ins/Number/prototype/toExponential/length.js
// Derived from: test/built-ins/Number/prototype/toExponential/range.js
if (Number.prototype.toExponential.length !== 1) {
  throw "expected Number.prototype.toExponential.length to be 1";
}
if (Number.prototype.toExponential() !== "0e+0") {
  throw "expected Number.prototype.toExponential() to return 0e+0";
}
if ((12.345).toExponential(2) !== "1.23e+1") {
  throw "expected toExponential to round fractional digits";
}
if ((1).toExponential(0) !== "1e+0") {
  throw "expected toExponential to allow zero fraction digits";
}
if (NaN.toExponential(101) !== "NaN") {
  throw "expected NaN toExponential to ignore digit range";
}
var caught = false;
try {
  (3).toExponential(101);
} catch (error) {
  caught = error instanceof RangeError;
}
if (!caught) {
  throw "expected toExponential to throw RangeError for fraction digits > 100";
}
