// Derived from: test/built-ins/Object/is/same-value-x-y-string.js
if (!Object.is("same", "same")) {
  throw "Object.is should treat identical strings as SameValue";
}
if (!Object.is(true, true)) {
  throw "Object.is should treat identical booleans as SameValue";
}
if (!Object.is(null, null)) {
  throw "Object.is should treat null values as SameValue";
}
if (!Object.is(undefined, undefined)) {
  throw "Object.is should treat undefined values as SameValue";
}
