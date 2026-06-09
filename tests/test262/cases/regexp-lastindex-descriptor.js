// Derived from: test/built-ins/RegExp/lastIndex.js

var re = new RegExp('');
var descriptor = Object.getOwnPropertyDescriptor(re, 'lastIndex');

if (re.lastIndex !== 0) {
  throw "expected constructed RegExp lastIndex to start at zero";
}
if (descriptor.value !== 0) {
  throw "expected constructed RegExp lastIndex descriptor value to be zero";
}
if (descriptor.writable !== true) {
  throw "expected constructed RegExp lastIndex to be writable";
}
if (descriptor.enumerable !== false) {
  throw "expected constructed RegExp lastIndex to be non-enumerable";
}
if (descriptor.configurable !== false) {
  throw "expected constructed RegExp lastIndex to be non-configurable";
}

var literal = /./;
var literalDescriptor = Object.getOwnPropertyDescriptor(literal, 'lastIndex');

if (literal.lastIndex !== 0) {
  throw "expected literal RegExp lastIndex to start at zero";
}
if (literalDescriptor.value !== 0) {
  throw "expected literal RegExp lastIndex descriptor value to be zero";
}
if (literalDescriptor.writable !== true) {
  throw "expected literal RegExp lastIndex to be writable";
}
if (literalDescriptor.enumerable !== false) {
  throw "expected literal RegExp lastIndex to be non-enumerable";
}
if (literalDescriptor.configurable !== false) {
  throw "expected literal RegExp lastIndex to be non-configurable";
}
