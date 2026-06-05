// Derived from: test/built-ins/Set/set-no-iterable.js
var set = new Set();
if (set.size !== 0) {
  throw "new Set without iterable should be empty";
}
set.add("a");
if (!set.has("a")) {
  throw "Set.prototype.has should find added values";
}
