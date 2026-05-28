// Derived from: test/built-ins/Object/entries/return-order.js
var entries = Object.entries([4, 5]);
if (entries.length !== 2 ||
    entries[0][0] !== "0" || entries[0][1] !== 4 ||
    entries[1][0] !== "1" || entries[1][1] !== 5) {
  throw "Object.entries should return array index entries";
}
