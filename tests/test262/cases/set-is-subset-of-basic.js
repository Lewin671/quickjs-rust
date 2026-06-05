// Derived from: test/built-ins/Set/prototype/isSubsetOf/compares-sets.js

if (new Set([1]).isSubsetOf(new Set([1, 2])) !== true) {
  throw "isSubsetOf should accept contained sets";
}
if (new Set([1, 3]).isSubsetOf(new Set([1, 2])) !== false) {
  throw "isSubsetOf should reject missing values";
}
