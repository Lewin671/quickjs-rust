// Derived from: test/built-ins/WeakMap/prototype/set/throw-if-key-cannot-be-held-weakly.js
// Derived from: test/built-ins/WeakSet/prototype/add/throw-when-value-cannot-be-held-weakly.js
function assertThrowsTypeError(callback, message) {
  var threw = false;
  try {
    callback();
  } catch (error) {
    threw = true;
    if (!(error instanceof TypeError)) {
      throw new Error(message + " threw " + error);
    }
  }
  if (!threw) {
    throw new Error(message + " did not throw");
  }
}

var mapSymbol = Symbol("map");
var map = new WeakMap();
map.set(mapSymbol, 1);
if (map.get(mapSymbol) !== 1) {
  throw new Error("WeakMap should accept non-registered symbols");
}
assertThrowsTypeError(function() {
  map.set(Symbol.for("map"), 2);
}, "WeakMap should reject registered symbols");

var setSymbol = Symbol("set");
var set = new WeakSet();
set.add(setSymbol);
if (!set.has(setSymbol)) {
  throw new Error("WeakSet should accept non-registered symbols");
}
assertThrowsTypeError(function() {
  set.add(Symbol.for("set"));
}, "WeakSet should reject registered symbols");
