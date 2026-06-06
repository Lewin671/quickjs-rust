// Derived from: test/built-ins/Object/getOwnPropertyDescriptors/symbols-included.js
var value = {};
var enumSym = Symbol('enum');
var nonEnumSym = Symbol('nonenum');
var symValue = Symbol('value');

var obj = {
  key: symValue
};
obj[enumSym] = value;
Object.defineProperty(obj, nonEnumSym, {
  writable: true,
  enumerable: false,
  configurable: true,
  value: value
});

var result = Object.getOwnPropertyDescriptors(obj);

if (Object.keys(result).length !== 1) {
  throw "expected one string-keyed descriptor";
}
if (Object.getOwnPropertySymbols(result).length !== 2) {
  throw "expected two symbol-keyed descriptors";
}
if (result.key.value !== symValue) {
  throw "expected string-keyed descriptor value";
}
if (result[enumSym].enumerable !== true) {
  throw "expected enumerable symbol descriptor";
}
if (result[enumSym].value !== value) {
  throw "expected enumerable symbol descriptor value";
}
if (result[nonEnumSym].enumerable !== false) {
  throw "expected non-enumerable symbol descriptor";
}
if (result[nonEnumSym].value !== value) {
  throw "expected non-enumerable symbol descriptor value";
}
