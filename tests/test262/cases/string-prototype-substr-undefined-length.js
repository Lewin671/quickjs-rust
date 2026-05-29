// Derived from: test/annexB/built-ins/String/prototype/substr/length-undef.js
if ("abc".substr(1) !== "bc") {
  throw "substr should read through the end when length is omitted";
}

if ("abc".substr(1, undefined) !== "bc") {
  throw "substr should read through the end when length is undefined";
}
