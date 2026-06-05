// Derived from: test/built-ins/RegExp/escape/is-function.js

if (typeof RegExp.escape !== "function") {
  throw "RegExp.escape should be a function";
}
if (RegExp.escape.length !== 1) {
  throw "RegExp.escape length should be 1";
}
if (RegExp.escape("abc123") !== "\\x61bc123") {
  throw "RegExp.escape should escape an initial ASCII letter";
}
if (RegExp.escape("1abc") !== "\\x31abc") {
  throw "RegExp.escape should escape an initial decimal digit";
}
if (RegExp.escape("_abc") !== "_abc") {
  throw "RegExp.escape should not escape underscore";
}
