// Derived from: test/built-ins/Object/fromEntries/supports-symbols.js
var key = Symbol();
var result = Object.fromEntries([[key, "value"]]);

if (result[key] !== "value") {
  throw "Object.fromEntries should support symbol keys";
}
