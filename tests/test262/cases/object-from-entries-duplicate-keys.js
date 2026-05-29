// Derived from: test/built-ins/Object/fromEntries/key-order.js
var result = Object.fromEntries([["key", 1], ["key", 2]]);
if (result.key !== 2) {
  throw "Object.fromEntries should let later entries overwrite earlier entries";
}
