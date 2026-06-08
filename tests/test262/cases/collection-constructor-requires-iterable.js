// Derived from: test/built-ins/WeakMap/iterable-failure.js
// Derived from: test/built-ins/WeakSet/iterable-failure.js
function assertThrowsTypeError(callback, message) {
  var caught = false;
  try {
    callback();
  } catch (error) {
    caught = error instanceof TypeError;
  }
  if (!caught) {
    throw new Error(message);
  }
}

assertThrowsTypeError(function() {
  new Map({});
}, "Map constructor should reject non-iterable objects");

assertThrowsTypeError(function() {
  new Set({});
}, "Set constructor should reject non-iterable objects");

assertThrowsTypeError(function() {
  new WeakMap({});
}, "WeakMap constructor should reject non-iterable objects");

assertThrowsTypeError(function() {
  new WeakSet({});
}, "WeakSet constructor should reject non-iterable objects");

var entries = [["a", 1], ["b", 2]];
var mapIterable = {};
mapIterable[Symbol.iterator] = function() {
  return entries[Symbol.iterator]();
};
var map = new Map(mapIterable);
if (map.get("b") !== 2) {
  throw new Error("Map constructor should consume iterable objects");
}

var values = ["x", "y"];
var setIterable = {};
setIterable[Symbol.iterator] = function() {
  return values[Symbol.iterator]();
};
var set = new Set(setIterable);
if (!set.has("x") || !set.has("y")) {
  throw new Error("Set constructor should consume iterable objects");
}
