// Derived from: test/built-ins/Object/fromEntries/simple-properties.js
var result = Object.fromEntries([["key", "value"]]);
if (result.key !== "value") {
  throw "Object.fromEntries should create properties from entries";
}
