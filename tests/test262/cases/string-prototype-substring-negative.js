// Derived from: test/built-ins/String/prototype/substring/S15.5.4.15_A1_T4.js
if ("gnulluna".substring(null, -3) !== "") {
  throw "expected null and negative indexes to clamp to 0";
}
if ("abcdef".substring(-3, 2) !== "ab") {
  throw "expected negative start to clamp to 0";
}
