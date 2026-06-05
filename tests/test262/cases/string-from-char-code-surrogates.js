// Derived from: test/built-ins/String/fromCharCode/S15.5.3.2_A1.js
var value = String.fromCharCode(0xd800, 0xdc00);
if (value.length !== 2) {
  throw "expected surrogate code units to be preserved";
}
if (value.charCodeAt(0) !== 0xd800) {
  throw "expected high surrogate code unit";
}
if (value.charCodeAt(1) !== 0xdc00) {
  throw "expected low surrogate code unit";
}
