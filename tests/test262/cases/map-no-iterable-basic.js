// Derived from: test/built-ins/Map/map-no-iterable.js
var map = new Map();
if (map.size !== 0) {
  throw "new Map without iterable should be empty";
}
map.set("a", 1);
if (map.get("a") !== 1) {
  throw "Map.prototype.get should retrieve stored values";
}

