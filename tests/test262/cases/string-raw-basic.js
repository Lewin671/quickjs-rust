// Derived from: test/built-ins/String/raw/length.js
if (typeof String.raw !== "function") {
  throw new Error("String.raw must be a function");
}
if (String.raw.length !== 1) {
  throw new Error("String.raw length must be 1");
}
if (String.propertyIsEnumerable("raw")) {
  throw new Error("String.raw must not be enumerable");
}
