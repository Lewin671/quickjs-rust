// Derived from: test/built-ins/Map/map-iterable.js
var map = new Map([
  ["attr", 1],
  ["foo", 2]
]);
if (map.size !== 2) {
  throw new Error("Map iterable constructor must populate size");
}
if (map.get("attr") !== 1 || map.get("foo") !== 2) {
  throw new Error("Map iterable constructor must populate entries");
}
