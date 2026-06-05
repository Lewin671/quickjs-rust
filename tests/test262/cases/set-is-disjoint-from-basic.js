// Derived from: test/built-ins/Set/prototype/isDisjointFrom/compares-sets.js

if (new Set([1, 2]).isDisjointFrom(new Set([3])) !== true) {
  throw "isDisjointFrom should accept disjoint sets";
}
if (new Set([1, 2]).isDisjointFrom(new Set([2, 3])) !== false) {
  throw "isDisjointFrom should reject shared values";
}
