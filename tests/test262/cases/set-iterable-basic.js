// Derived from: test/built-ins/Set/set-iterable.js
var set = new Set([1, 2]);
if (set.size !== 2) {
  throw new Error("Set iterable constructor must populate size");
}
if (!set.has(1) || !set.has(2)) {
  throw new Error("Set iterable constructor must populate values");
}
