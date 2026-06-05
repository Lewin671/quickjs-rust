// Derived from: test/built-ins/String/prototype/toWellFormed/length.js
if (String.prototype.toWellFormed.length !== 0) {
  throw new Error("toWellFormed length must be 0");
}
if (String.prototype.propertyIsEnumerable("toWellFormed")) {
  throw new Error("toWellFormed must not be enumerable");
}
