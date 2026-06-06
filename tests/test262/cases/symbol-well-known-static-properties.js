// Derived from: test/built-ins/Symbol/asyncDispose/prop-desc.js
// Derived from: test/built-ins/Symbol/asyncIterator/prop-desc.js
// Derived from: test/built-ins/Symbol/dispose/prop-desc.js
// Derived from: test/built-ins/Symbol/hasInstance/prop-desc.js
// Derived from: test/built-ins/Symbol/isConcatSpreadable/prop-desc.js
// Derived from: test/built-ins/Symbol/match/prop-desc.js
// Derived from: test/built-ins/Symbol/matchAll/prop-desc.js
// Derived from: test/built-ins/Symbol/replace/prop-desc.js
// Derived from: test/built-ins/Symbol/search/prop-desc.js
// Derived from: test/built-ins/Symbol/species/prop-desc.js
// Derived from: test/built-ins/Symbol/split/prop-desc.js
// Derived from: test/built-ins/Symbol/toPrimitive/prop-desc.js
// Derived from: test/built-ins/Symbol/unscopables/prop-desc.js
var names = [
  "asyncDispose",
  "asyncIterator",
  "dispose",
  "hasInstance",
  "isConcatSpreadable",
  "match",
  "matchAll",
  "replace",
  "search",
  "species",
  "split",
  "toPrimitive",
  "unscopables",
];

for (var index = 0; index < names.length; index++) {
  var name = names[index];
  var descriptor = Object.getOwnPropertyDescriptor(Symbol, name);
  if (typeof Symbol[name] !== "symbol") {
    throw "expected Symbol." + name + " to be a symbol";
  }
  if (String(Symbol[name]) !== "Symbol(Symbol." + name + ")") {
    throw "expected Symbol." + name + " descriptive string";
  }
  if (descriptor.writable !== false) {
    throw "expected Symbol." + name + " to be non-writable";
  }
  if (descriptor.enumerable !== false) {
    throw "expected Symbol." + name + " to be non-enumerable";
  }
  if (descriptor.configurable !== false) {
    throw "expected Symbol." + name + " to be non-configurable";
  }
  if (Symbol.keyFor(Symbol[name]) !== undefined) {
    throw "expected Symbol." + name + " to be outside the global registry";
  }
}
