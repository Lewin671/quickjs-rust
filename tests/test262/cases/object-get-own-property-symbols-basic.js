// Derived from: test/built-ins/Object/getOwnPropertySymbols/object-contains-symbol-property-with-description.js
var first = Symbol("first");
var second = Symbol("second");
var object = {};

Object.defineProperty(object, first, {
  value: 11,
  enumerable: true,
  configurable: true
});
Object.defineProperty(object, second, {
  value: 22
});

var symbols = Object.getOwnPropertySymbols(object);

if (Object.getOwnPropertySymbols.length !== 1) { throw; }
if (symbols.length !== 2) { throw; }
if (symbols[0] !== first) { throw; }
if (symbols[1] !== second) { throw; }
if (Object.getOwnPropertyDescriptor(object, symbols[0]).value !== 11) { throw; }
if (Object.hasOwn(object, second) !== true) { throw; }
if (Object.getOwnPropertyNames(object).length !== 0) { throw; }
