// Derived from: test/built-ins/String/prototype/codePointAt/returns-undefined-on-position-equal-or-more-than-size.js
if ("abc".codePointAt(3) !== undefined) {
  throw "expected codePointAt at string length to return undefined";
}
if ("abc".codePointAt(Infinity) !== undefined) {
  throw "expected codePointAt at Infinity to return undefined";
}
if ("abc".codePointAt(-1) !== undefined) {
  throw "expected negative codePointAt index to return undefined";
}
