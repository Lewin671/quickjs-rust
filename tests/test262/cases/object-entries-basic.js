// Derived from: test/built-ins/Object/entries/return-order.js
var entries = Object.entries({ first: 1, second: 2 });
if (entries.length !== 2 ||
    entries[0][0] !== "first" || entries[0][1] !== 1 ||
    entries[1][0] !== "second" || entries[1][1] !== 2) {
  throw "Object.entries should return own enumerable key-value entries";
}
