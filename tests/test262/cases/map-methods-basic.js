// Derived from: test/built-ins/Map/valid-keys.js
var objectKey = {};
var map = new Map();
map.set("a", 1);
map.set("a", 2);
map.set(NaN, 3);
map.set(-0, 4);
map.set(0, 5);
map.set(objectKey, 6);
if (map.size !== 4) {
  throw "Map should use SameValueZero for key replacement";
}
if (map.get("a") !== 2 || map.get(NaN) !== 3 || map.get(-0) !== 5 || map.get(objectKey) !== 6) {
  throw "Map should return values for supported key types";
}
if (map.has({})) {
  throw "Map object keys should use identity";
}
if (!map.delete("a") || map.delete("a") || map.has("a")) {
  throw "Map.prototype.delete should report and remove keys";
}
map.clear();
if (map.size !== 0) {
  throw "Map.prototype.clear should remove entries";
}

