// Derived from: test/built-ins/String/S15.5.1.1_A1_T1.js
if (typeof String(42) !== "string") {
  throw "expected String(42) to produce a string";
}
if (String() !== "") {
  throw "expected String() to return an empty string";
}
if (String(42) !== "42") {
  throw "expected String(42) to return '42'";
}
if (String(null) !== "null") {
  throw "expected String(null) to return 'null'";
}
if (String(undefined) !== "undefined") {
  throw "expected String(undefined) to return 'undefined'";
}
