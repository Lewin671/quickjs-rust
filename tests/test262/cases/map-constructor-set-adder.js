// Derived from: test/built-ins/Map/iterable-calls-set.js
var originalSet = Map.prototype.set;
var calls = 0;
var receiver;
Map.prototype.set = function(key, value) {
  calls++;
  receiver = this;
  return originalSet.call(this, key, value);
};
var map = new Map([["a", 1]]);
if (calls !== 1 || receiver !== map || map.get("a") !== 1) {
  throw new Error("Map constructor should call the prototype set adder");
}

// Derived from: test/built-ins/Map/map-iterable-throws-when-set-is-not-callable.js
Map.prototype.set = null;
var caught = false;
try {
  new Map([["b", 2]]);
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught) {
  throw new Error("Map constructor should reject non-callable set adders");
}
