// Derived from: test/built-ins/Number/NaN.js
if (Number.NaN === Number.NaN) {
  throw "expected Number.NaN to be NaN";
}
if (Number.POSITIVE_INFINITY !== Infinity) {
  throw "expected Number.POSITIVE_INFINITY to be Infinity";
}
if (Number.NEGATIVE_INFINITY !== -Infinity) {
  throw "expected Number.NEGATIVE_INFINITY to be -Infinity";
}
if (Number.MAX_SAFE_INTEGER !== 9007199254740991) {
  throw "expected Number.MAX_SAFE_INTEGER value";
}
if (Number.MIN_SAFE_INTEGER !== -9007199254740991) {
  throw "expected Number.MIN_SAFE_INTEGER value";
}
if (Object.getOwnPropertyDescriptor(Number, "NaN").writable) {
  throw "expected Number.NaN to be non-writable";
}
