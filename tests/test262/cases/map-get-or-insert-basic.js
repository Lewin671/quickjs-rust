// Derived from: test/built-ins/Map/prototype/getOrInsert/append-value-if-key-is-not-present-different-key-types.js

var map = new Map();

if (map.getOrInsert("a", 1) !== 1) {
  throw "getOrInsert should return the inserted value";
}
if (map.get("a") !== 1) {
  throw "getOrInsert should store the inserted value";
}
if (map.getOrInsert("a", 2) !== 1) {
  throw "getOrInsert should return the existing value";
}
if (map.get("a") !== 1) {
  throw "getOrInsert should not overwrite an existing value";
}
if (Map.prototype.getOrInsert.length !== 2) {
  throw "getOrInsert length should be 2";
}
