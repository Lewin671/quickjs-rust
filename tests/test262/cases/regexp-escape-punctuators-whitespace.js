// Derived from: test/built-ins/RegExp/escape/escaped-syntax-characters-simple.js

if (RegExp.escape("^$\\.*+?()[]{}|/") !== "\\^\\$\\\\\\.\\*\\+\\?\\(\\)\\[\\]\\{\\}\\|\\/") {
  throw "RegExp.escape should escape syntax characters";
}
if (RegExp.escape(",-=<>#&!%:;@~'`\"") !== "\\x2c\\x2d\\x3d\\x3c\\x3e\\x23\\x26\\x21\\x25\\x3a\\x3b\\x40\\x7e\\x27\\x60\\x22") {
  throw "RegExp.escape should hex-escape other punctuators";
}
if (RegExp.escape("\t\n\v\f\r ") !== "\\t\\n\\v\\f\\r\\x20") {
  throw "RegExp.escape should escape control whitespace";
}
if (RegExp.escape(String.fromCharCode(0x00a0, 0x2028, 0xfeff)) !== "\\xa0\\u2028\\ufeff") {
  throw "RegExp.escape should escape non-ASCII whitespace";
}
if (RegExp.escape("\ud800\udc00") !== "\\ud800\\udc00") {
  throw "RegExp.escape should escape surrogate code units";
}
