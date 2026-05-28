// Derived from: test/built-ins/Object/prototype/toLocaleString/S15.2.4.3_A11.js
if (Object.prototype.toLocaleString.length !== 0) {
  throw new Error("bad length");
}
if (!Object.prototype.toLocaleString.hasOwnProperty("length")) {
  throw new Error("missing length own property");
}
if (Object.prototype.toLocaleString.propertyIsEnumerable("length")) {
  throw new Error("length should be non-enumerable");
}
