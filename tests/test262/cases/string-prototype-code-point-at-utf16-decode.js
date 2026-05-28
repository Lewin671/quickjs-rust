// Derived from: test/built-ins/String/prototype/codePointAt/return-utf16-decode.js
if ("😀".codePointAt(0) !== 128512) {
  throw "expected codePointAt(0) to decode a UTF-16 surrogate pair";
}
if ("😀".codePointAt(1) !== 56832) {
  throw "expected codePointAt(1) to return the trailing surrogate code unit";
}
