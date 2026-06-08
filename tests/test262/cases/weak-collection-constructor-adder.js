// Derived from: test/built-ins/WeakMap/get-set-method-failure.js
// Derived from: test/built-ins/WeakMap/iterable-with-object-keys.js
// Derived from: test/built-ins/WeakSet/get-add-method-failure.js
// Derived from: test/built-ins/WeakSet/iterable-with-object-values.js
var mapKey = {};
var mapCalls = 0;
var originalSet = WeakMap.prototype.set;
WeakMap.prototype.set = function(key, value) {
  mapCalls = mapCalls + 1;
  return originalSet.call(this, key, value);
};
var map = new WeakMap([[mapKey, 7]]);
if (mapCalls !== 1 || map.get(mapKey) !== 7) {
  throw "WeakMap constructor should call the prototype set adder";
}

var setValue = {};
var setCalls = 0;
var originalAdd = WeakSet.prototype.add;
WeakSet.prototype.add = function(value) {
  setCalls = setCalls + 1;
  return originalAdd.call(this, value);
};
var set = new WeakSet([setValue]);
if (setCalls !== 1 || !set.has(setValue)) {
  throw "WeakSet constructor should call the prototype add adder";
}

Object.defineProperty(WeakMap.prototype, "set", {
  get: function() {
    throw new TypeError("WeakMap set getter");
  }
});
var mapGetterThrows = false;
try {
  new WeakMap([]);
} catch (error) {
  mapGetterThrows = error.constructor === TypeError;
}
if (!mapGetterThrows) {
  throw "WeakMap constructor should propagate set getter abrupt completions";
}

Object.defineProperty(WeakSet.prototype, "add", {
  get: function() {
    throw new TypeError("WeakSet add getter");
  }
});
var setGetterThrows = false;
try {
  new WeakSet([]);
} catch (error) {
  setGetterThrows = error.constructor === TypeError;
}
if (!setGetterThrows) {
  throw "WeakSet constructor should propagate add getter abrupt completions";
}
