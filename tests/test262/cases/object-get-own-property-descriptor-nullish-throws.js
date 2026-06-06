// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-1-1.js
// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-1-2.js
// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-1-3.js
try {
  Object.getOwnPropertyDescriptor(undefined, "foo");
  throw "undefined target should throw";
} catch (error) {
  if (!(error instanceof TypeError)) {
    throw "undefined target should throw TypeError";
  }
}

try {
  Object.getOwnPropertyDescriptor(null, "foo");
  throw "null target should throw";
} catch (error) {
  if (!(error instanceof TypeError)) {
    throw "null target should throw TypeError";
  }
}

if (Object.getOwnPropertyDescriptor(1, "foo") !== undefined) {
  throw "number target should coerce to object";
}
