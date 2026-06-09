// Derived from: test/built-ins/RegExp/prototype/exec/u-lastindex-value.js

var r = /./ug;
var result = r.exec("\uD834\uDF06");

if (result === null) {
  throw "expected unicode dot to match astral character";
}
if (result.index !== 0) {
  throw "expected match index to use UTF-16 code units";
}
if (result[0].length !== 2) {
  throw "expected match string length to use UTF-16 code units";
}
if (r.lastIndex !== 2) {
  throw "expected unicode global RegExp lastIndex to advance by UTF-16 width";
}
