// Derived from: test/built-ins/Array/Symbol.species/symbol-species.js
// Derived from: test/built-ins/Array/Symbol.species/return-value.js
// Derived from: test/built-ins/Array/Symbol.species/length.js
// Derived from: test/built-ins/Array/Symbol.species/symbol-species-name.js
// Derived from: test/built-ins/Map/Symbol.species/symbol-species.js
// Derived from: test/built-ins/Map/Symbol.species/return-value.js
// Derived from: test/built-ins/Map/Symbol.species/length.js
// Derived from: test/built-ins/Map/Symbol.species/symbol-species-name.js
// Derived from: test/built-ins/Set/Symbol.species/symbol-species.js
// Derived from: test/built-ins/Set/Symbol.species/return-value.js
// Derived from: test/built-ins/Set/Symbol.species/length.js
// Derived from: test/built-ins/Set/Symbol.species/symbol-species-name.js
function verifySpeciesAccessor(ctor, name) {
  var desc = Object.getOwnPropertyDescriptor(ctor, Symbol.species);
  var receiver = {};
  if (desc.get.call(receiver) !== receiver) {
    throw new Error(name + " species getter should return its receiver");
  }
  if (desc.set !== undefined) {
    throw new Error(name + " species setter should be undefined");
  }
  if (desc.enumerable !== false || desc.configurable !== true) {
    throw new Error(name + " species descriptor attributes should match spec");
  }
  if (desc.get.name !== "get [Symbol.species]" || desc.get.length !== 0) {
    throw new Error(name + " species getter metadata should match spec");
  }
}

verifySpeciesAccessor(Array, "Array");
verifySpeciesAccessor(Map, "Map");
verifySpeciesAccessor(Set, "Set");
