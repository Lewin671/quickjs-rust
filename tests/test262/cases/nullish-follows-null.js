// Derived from: test/language/expressions/coalesce/follows-null.js
var x;

x = null ?? 42;
if (x !== 42) {
  throw;
}

x = null ?? undefined;
if (x !== undefined) {
  throw;
}

x = null ?? null;
if (x !== null) {
  throw;
}

x = null ?? false;
if (x !== false) {
  throw;
}
