// Derived from: test/built-ins/String/fromCharCode/S15.5.3.2_A1.js
if (String.fromCharCode(65, 66, 67) !== "ABC") {
  throw "expected String.fromCharCode to convert code units";
}
if (String.fromCharCode() !== "") {
  throw "expected String.fromCharCode() to return an empty string";
}
