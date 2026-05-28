// Derived from: test/built-ins/String/prototype/substring/S15.5.4.15_A1_T6.js
if ("undefined".substring(undefined, 3) !== "und") {
  throw "expected undefined start to be treated as 0";
}
if ("abcdef".substring(1, 4) !== "bcd") {
  throw "expected substring to return selected range";
}
