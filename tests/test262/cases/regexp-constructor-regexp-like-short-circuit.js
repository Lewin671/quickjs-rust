// Derived from: test/built-ins/RegExp/from-regexp-like-short-circuit.js
var obj = {
  constructor: RegExp
};

obj[Symbol.match] = true;

if (RegExp(obj) !== obj) {
  throw "RegExp should return a regexp-like argument with the same constructor";
}
