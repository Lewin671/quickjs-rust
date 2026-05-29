// Derived from: test/built-ins/Object/fromEntries/string-entry-object-succeeds.js
var entry = { 0: "key", 1: "value" };
var result = Object.fromEntries([entry]);
if (result.key !== "value") {
  throw "Object.fromEntries should read entry keys 0 and 1";
}
