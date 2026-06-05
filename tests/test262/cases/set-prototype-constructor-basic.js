// Derived from: test/built-ins/Set/prototype/constructor/set-prototype-constructor.js
if (Set.prototype.constructor !== Set) {
  throw "Set.prototype.constructor should reference Set";
}
var descriptor = Object.getOwnPropertyDescriptor(Set.prototype, "constructor");
if (!descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "Set.prototype.constructor descriptor should match ordinary constructor links";
}
