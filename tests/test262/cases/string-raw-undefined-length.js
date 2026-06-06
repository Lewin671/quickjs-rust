// Derived from: test/built-ins/String/raw/return-empty-string-if-length-is-not-defined.js
// Derived from: test/built-ins/String/raw/return-empty-string-if-length-is-undefined.js
if (String.raw({ raw: {} }) !== "") {
  throw "expected missing raw length to return an empty string";
}
if (String.raw({ raw: { length: undefined } }) !== "") {
  throw "expected undefined raw length to return an empty string";
}
