// Derived from: test/built-ins/Object/is/not-same-value-x-y-number.js
if (Object.is(+0, -0)) {
  throw "Object.is should distinguish +0 and -0";
}
if (Object.is(-0, +0)) {
  throw "Object.is should distinguish -0 and +0";
}
if (Object.is(0)) {
  throw "Object.is should compare missing second argument as undefined";
}
if (Object.is(Infinity, -Infinity)) {
  throw "Object.is should reject different infinities";
}
