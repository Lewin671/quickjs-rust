// Derived from: test/built-ins/Object/is/same-value-x-y-number.js
if (!Object.is(NaN, NaN)) {
  throw "Object.is should treat NaN as SameValue";
}
if (!Object.is(-0, -0)) {
  throw "Object.is should treat -0 and -0 as SameValue";
}
if (!Object.is(+0, +0)) {
  throw "Object.is should treat +0 and +0 as SameValue";
}
if (!Object.is(1, 1)) {
  throw "Object.is should treat equal numbers as SameValue";
}
