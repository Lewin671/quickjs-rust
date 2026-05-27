// Derived from: test/language/expressions/coalesce/follows-undefined.js
var x;

x = undefined ?? 42;
if (x !== 42) {
  throw;
}

x = undefined ?? null;
if (x !== null) {
  throw;
}

x = undefined ?? false;
if (x !== false) {
  throw;
}
