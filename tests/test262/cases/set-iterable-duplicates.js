// Derived from: test/built-ins/Set/prototype/add/will-not-add-duplicate-entry-initial-iterable.js
var set = new Set([1, 1, 2]);
if (set.size !== 2) {
  throw new Error("Set iterable constructor must ignore duplicate values");
}
