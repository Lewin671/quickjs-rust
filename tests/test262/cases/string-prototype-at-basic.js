// Derived from: test/built-ins/String/prototype/at/returns-item-relative-index.js
if ("abc".at(1) !== "b") {
  throw "expected positive index to return matching item";
}
if ("abc".at(-1) !== "c") {
  throw "expected negative index to return relative item";
}
if ("abc".at(3) !== undefined) {
  throw "expected out-of-range positive index to return undefined";
}
if ("abc".at(-4) !== undefined) {
  throw "expected out-of-range negative index to return undefined";
}
if ("abc".at() !== "a") {
  throw "expected omitted index to use zero";
}
if ("abc".at(1.9) !== "b") {
  throw "expected index to be truncated";
}
if (String.prototype.at.length !== 1) {
  throw "expected String.prototype.at.length to be 1";
}
if (String.prototype.at.propertyIsEnumerable("length")) {
  throw "expected String.prototype.at.length to be non-enumerable";
}
