// Derived from: test/built-ins/Function/prototype/Symbol.hasInstance/prop-desc.js
// Derived from: test/built-ins/Function/prototype/Symbol.hasInstance/length.js
// Derived from: test/built-ins/Function/prototype/Symbol.hasInstance/name.js
// Derived from: test/built-ins/Function/prototype/Symbol.hasInstance/value-positive.js
// Derived from: test/built-ins/Function/prototype/Symbol.hasInstance/value-negative.js
// Derived from: test/built-ins/Function/prototype/Symbol.hasInstance/this-val-non-callable.js
// Derived from: test/language/expressions/instanceof/symbol-hasinstance-invocation.js
// Derived from: test/language/expressions/instanceof/symbol-hasinstance-to-boolean.js
// Derived from: test/language/expressions/instanceof/symbol-hasinstance-not-callable.js
function C() {}
var instance = new C();
var descriptor = Object.getOwnPropertyDescriptor(Function.prototype, Symbol.hasInstance);
if (typeof descriptor.value !== "function") {
  throw "expected Function.prototype[Symbol.hasInstance] to be a function";
}
if (descriptor.writable !== false) {
  throw "expected Symbol.hasInstance to be non-writable";
}
if (descriptor.enumerable !== false) {
  throw "expected Symbol.hasInstance to be non-enumerable";
}
if (descriptor.configurable !== false) {
  throw "expected Symbol.hasInstance to be non-configurable";
}
if (descriptor.value.length !== 1) {
  throw "expected Symbol.hasInstance length";
}
if (descriptor.value.name !== "[Symbol.hasInstance]") {
  throw "expected Symbol.hasInstance name";
}
if (descriptor.value.call(C, instance) !== true) {
  throw "expected builtin hasInstance to accept instances";
}
if (descriptor.value.call(C, {}) !== false) {
  throw "expected builtin hasInstance to reject unrelated objects";
}
if (descriptor.value.call({}, {}) !== false) {
  throw "expected builtin hasInstance to return false for non-callable this";
}

var calls = 0;
var matcher = {};
matcher[Symbol.hasInstance] = function (value) {
  calls = calls + (this === matcher ? 1 : 0);
  return value === 7;
};
if ((7 instanceof matcher) !== true) {
  throw "expected custom object hasInstance true";
}
if ((8 instanceof matcher) !== false) {
  throw "expected custom object hasInstance false";
}
if (calls !== 2) {
  throw "expected custom object hasInstance calls";
}

function FunctionMatcher() {}
Object.defineProperty(FunctionMatcher, Symbol.hasInstance, {
  value: function (value) {
    return value === 3 ? "yes" : "";
  },
  configurable: true,
});
if ((3 instanceof FunctionMatcher) !== true) {
  throw "expected custom function hasInstance truthy result";
}
if ((4 instanceof FunctionMatcher) !== false) {
  throw "expected custom function hasInstance falsy result";
}

var notCallable = {};
notCallable[Symbol.hasInstance] = 1;
var caught = false;
try {
  1 instanceof notCallable;
} catch (error) {
  caught = error instanceof TypeError;
}
if (caught !== true) {
  throw "expected non-callable Symbol.hasInstance to throw TypeError";
}
