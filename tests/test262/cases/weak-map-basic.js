// Derived from: test/built-ins/WeakMap/constructor.js
var key = {};
var other = {};
var array = [];
var fn = function() {};
var map = new WeakMap([[key, 1]]);
map.set(key, 2);
map.set(array, 3);
map.set(fn, 4);
if (!(map instanceof WeakMap)) {
  throw "WeakMap constructor should create WeakMap instances";
}
if (Object.prototype.toString.call(map) !== "[object WeakMap]") {
  throw "WeakMap should expose the WeakMap toString tag";
}
if (WeakMap.prototype.size !== undefined) {
  throw "WeakMap.prototype should not expose size";
}
if (map.get(key) !== 2 || map.get(array) !== 3 || map.get(fn) !== 4) {
  throw "WeakMap should store object keys by identity";
}
if (map.has(other)) {
  throw "WeakMap object keys should use identity";
}
if (!map.delete(key) || map.delete(key) || map.has(key)) {
  throw "WeakMap.prototype.delete should report and remove object keys";
}
if (map.get("key") !== undefined || map.has("key") || map.delete("key")) {
  throw "WeakMap get, has, and delete should tolerate primitive keys";
}
var primitiveSetThrows = false;
try {
  map.set("key", 5);
} catch (error) {
  primitiveSetThrows = error.constructor === TypeError;
}
if (!primitiveSetThrows) {
  throw "WeakMap.prototype.set should reject primitive keys";
}
