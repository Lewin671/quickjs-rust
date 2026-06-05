// Derived from: test/built-ins/Map/prototype/entries/entries.js
var map = new Map();
map.set("a", 1);
map.set("b", 2);
var entries = map.entries();
var first = entries.next();
var second = entries.next();
var last = entries.next();
if (first.done || first.value[0] !== "a" || first.value[1] !== 1) {
  throw "Map.prototype.entries should yield key-value pairs";
}
if (second.done || second.value[0] !== "b" || second.value[1] !== 2) {
  throw "Map.prototype.entries should preserve insertion order";
}
if (!last.done || last.value !== undefined) {
  throw "Map iterator should finish with undefined value";
}
if (map.keys().next().value !== "a" || map.values().next().value !== 1) {
  throw "Map keys and values iterators should yield the expected fields";
}
