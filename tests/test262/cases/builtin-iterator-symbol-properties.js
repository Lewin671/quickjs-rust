// Derived from: test/built-ins/Symbol/iterator/prop-desc.js
// Derived from: test/built-ins/Array/prototype/Symbol.iterator.js
// Derived from: test/built-ins/Map/prototype/Symbol.iterator.js
// Derived from: test/built-ins/Set/prototype/Symbol.iterator.js
var symbolDescriptor = Object.getOwnPropertyDescriptor(Symbol, "iterator");
if (typeof Symbol.iterator !== "symbol") {
  throw "expected Symbol.iterator to be a symbol";
}
if (symbolDescriptor.writable !== false) {
  throw "expected Symbol.iterator to be non-writable";
}
if (symbolDescriptor.enumerable !== false) {
  throw "expected Symbol.iterator to be non-enumerable";
}
if (symbolDescriptor.configurable !== false) {
  throw "expected Symbol.iterator to be non-configurable";
}

function verifyIteratorAlias(object, method) {
  var descriptor = Object.getOwnPropertyDescriptor(object, Symbol.iterator);
  if (descriptor.value !== object[method]) {
    throw "expected Symbol.iterator to alias " + method;
  }
  if (descriptor.writable !== true) {
    throw "expected Symbol.iterator alias to be writable";
  }
  if (descriptor.enumerable !== false) {
    throw "expected Symbol.iterator alias to be non-enumerable";
  }
  if (descriptor.configurable !== true) {
    throw "expected Symbol.iterator alias to be configurable";
  }
}

verifyIteratorAlias(Array.prototype, "values");
verifyIteratorAlias(Map.prototype, "entries");
verifyIteratorAlias(Set.prototype, "values");
