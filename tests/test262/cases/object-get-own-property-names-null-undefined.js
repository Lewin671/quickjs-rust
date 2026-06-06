// Derived from: test/built-ins/Object/getOwnPropertyNames/non-object-argument-invalid.js
var count = 0;

try {
  count++;
  Object.getOwnPropertyNames(undefined);
  throw "expected undefined target to throw";
} catch (error) {
  if (!(error instanceof TypeError)) {
    throw "expected TypeError for undefined target";
  }
}

try {
  count++;
  Object.getOwnPropertyNames(null);
  throw "expected null target to throw";
} catch (error) {
  if (!(error instanceof TypeError)) {
    throw "expected TypeError for null target";
  }
}

if (count !== 2) {
  throw "expected both calls to be evaluated";
}
