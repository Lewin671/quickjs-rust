// Derived from: test/built-ins/Symbol/prototype/Symbol.toStringTag.js
// Derived from: test/built-ins/Map/prototype/Symbol.toStringTag.js
// Derived from: test/built-ins/Set/prototype/Symbol.toStringTag.js
// Derived from: test/built-ins/WeakMap/prototype/Symbol.toStringTag.js
// Derived from: test/built-ins/WeakSet/prototype/Symbol.toStringTag.js
// Derived from: test/built-ins/Promise/prototype/Symbol.toStringTag.js
// Derived from: test/built-ins/Math/Symbol.toStringTag.js
// Derived from: test/built-ins/JSON/Symbol.toStringTag.js
function verifyTag(object, expected) {
  var descriptor = Object.getOwnPropertyDescriptor(object, Symbol.toStringTag);
  if (descriptor.value !== expected) {
    throw "unexpected Symbol.toStringTag value for " + expected;
  }
  if (descriptor.writable !== false) {
    throw "expected Symbol.toStringTag to be non-writable for " + expected;
  }
  if (descriptor.enumerable !== false) {
    throw "expected Symbol.toStringTag to be non-enumerable for " + expected;
  }
  if (descriptor.configurable !== true) {
    throw "expected Symbol.toStringTag to be configurable for " + expected;
  }
}

verifyTag(Symbol.prototype, "Symbol");
verifyTag(Map.prototype, "Map");
verifyTag(Set.prototype, "Set");
verifyTag(WeakMap.prototype, "WeakMap");
verifyTag(WeakSet.prototype, "WeakSet");
verifyTag(Promise.prototype, "Promise");
verifyTag(Math, "Math");
verifyTag(JSON, "JSON");
