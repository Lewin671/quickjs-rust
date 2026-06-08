// Derived from: test/built-ins/Reflect/defineProperty/target-is-symbol-throws.js
// Derived from: test/built-ins/Reflect/deleteProperty/target-is-symbol-throws.js
// Derived from: test/built-ins/Reflect/get/target-is-symbol-throws.js
// Derived from: test/built-ins/Reflect/getOwnPropertyDescriptor/target-is-symbol-throws.js
// Derived from: test/built-ins/Reflect/has/target-is-symbol-throws.js
// Derived from: test/built-ins/Reflect/isExtensible/target-is-symbol-throws.js
// Derived from: test/built-ins/Reflect/ownKeys/target-is-symbol-throws.js
// Derived from: test/built-ins/Reflect/preventExtensions/target-is-symbol-throws.js
// Derived from: test/built-ins/Reflect/set/target-is-symbol-throws.js
function assertTypeError(callback) {
  try {
    callback();
  } catch (error) {
    if (error instanceof TypeError) {
      return;
    }
    throw "expected TypeError";
  }
  throw "expected throw";
}

assertTypeError(function() {
  Reflect.defineProperty(Symbol("target"), "key", {});
});
assertTypeError(function() {
  Reflect.deleteProperty(Symbol("target"), "key");
});
assertTypeError(function() {
  Reflect.get(Symbol("target"), "key");
});
assertTypeError(function() {
  Reflect.getOwnPropertyDescriptor(Symbol("target"), "key");
});
assertTypeError(function() {
  Reflect.has(Symbol("target"), "key");
});
assertTypeError(function() {
  Reflect.isExtensible(Symbol("target"));
});
assertTypeError(function() {
  Reflect.ownKeys(Symbol("target"));
});
assertTypeError(function() {
  Reflect.preventExtensions(Symbol("target"));
});
assertTypeError(function() {
  Reflect.set(Symbol("target"), "key", 1);
});
