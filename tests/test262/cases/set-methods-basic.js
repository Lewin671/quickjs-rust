// Derived from: test/built-ins/Set/prototype/add/returns-this.js
var value = {};
var set = new Set();
if (set.add("a") !== set) {
  throw "Set.prototype.add should return this";
}
set.add("a");
set.add(NaN);
set.add(NaN);
set.add(-0);
set.add(0);
set.add(value);
if (set.size !== 4) {
  throw "Set should use SameValueZero for value replacement";
}
if (!set.has("a") || set.has("b") || !set.has(NaN) || !set.has(-0) || !set.has(0)) {
  throw "Set.prototype.has should find supported value types";
}
if (!set.has(value) || set.has({})) {
  throw "Set object values should use identity";
}
if (!set.delete("a") || set.delete("a") || set.has("a")) {
  throw "Set.prototype.delete should report and remove values";
}
set.clear();
if (set.size !== 0) {
  throw "Set.prototype.clear should remove entries";
}
