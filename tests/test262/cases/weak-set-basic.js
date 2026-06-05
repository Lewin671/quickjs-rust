// Derived from: test/built-ins/WeakSet/constructor.js
// Derived from: test/built-ins/WeakSet/no-iterable.js
// Derived from: test/built-ins/WeakSet/prototype/add/returns-this-when-ignoring-duplicate.js
// Derived from: test/built-ins/WeakSet/prototype/delete/returns-false-if-value-not-object.js
// Derived from: test/built-ins/WeakSet/prototype/has/returns-false-if-value-not-object.js
var key = {};
var other = {};
var array = [];
var fn = function() {};
var set = new WeakSet([key]);

if (typeof WeakSet !== "function") {
  throw "WeakSet should be a function";
}
if (!(set instanceof WeakSet)) {
  throw "WeakSet constructor should create WeakSet instances";
}
if (Object.prototype.toString.call(set) !== "[object WeakSet]") {
  throw "WeakSet should expose the WeakSet toString tag";
}
if (WeakSet.prototype.size !== undefined) {
  throw "WeakSet.prototype should not expose size";
}
if (set.add(key) !== set) {
  throw "WeakSet.prototype.add should return the receiver";
}
set.add(array);
set.add(fn);
if (!set.has(key) || !set.has(array) || !set.has(fn)) {
  throw "WeakSet should store object values by identity";
}
if (set.has(other)) {
  throw "WeakSet object values should use identity";
}
if (!set.delete(key) || set.has(key)) {
  throw "WeakSet.prototype.delete should report and remove object values";
}
if (set.has("key") || set.delete("key")) {
  throw "WeakSet has and delete should tolerate primitive values";
}
var addPrimitiveThrows = false;
try {
  set.add("key");
} catch (error) {
  addPrimitiveThrows = error.constructor === TypeError;
}
if (!addPrimitiveThrows) {
  throw "WeakSet.prototype.add should reject primitive values";
}
var constructorPrimitiveThrows = false;
try {
  new WeakSet([1]);
} catch (error) {
  constructorPrimitiveThrows = error.constructor === TypeError;
}
if (!constructorPrimitiveThrows) {
  throw "WeakSet constructor should reject primitive entries";
}
