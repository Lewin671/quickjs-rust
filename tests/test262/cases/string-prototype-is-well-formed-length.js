// Derived from: test/built-ins/String/prototype/isWellFormed/length.js
if (String.prototype.isWellFormed.length !== 0) {
  throw new Error("isWellFormed length must be 0");
}
if (String.prototype.propertyIsEnumerable("isWellFormed")) {
  throw new Error("isWellFormed must not be enumerable");
}
