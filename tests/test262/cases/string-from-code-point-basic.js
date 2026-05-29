// Derived from: test/built-ins/String/fromCodePoint/return-string-value.js
if (String.fromCodePoint(65, 128512, 67) !== "A😀C") {
  throw "expected String.fromCodePoint to encode code points";
}
if (String.fromCodePoint() !== "") {
  throw "expected String.fromCodePoint() to return an empty string";
}
