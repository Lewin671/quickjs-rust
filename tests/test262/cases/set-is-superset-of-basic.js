// Derived from: test/built-ins/Set/prototype/isSupersetOf/compares-sets.js

if (new Set([1, 2]).isSupersetOf(new Set([1])) !== true) {
  throw "isSupersetOf should accept contained sets";
}
if (new Set([1, 2]).isSupersetOf(new Set([1, 3])) !== false) {
  throw "isSupersetOf should reject missing values";
}
