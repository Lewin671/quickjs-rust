// Derived from: test/built-ins/Math/PI/value.js
if (typeof Math.PI !== "number") {
  throw "expected Math.PI to be a number";
}

if (Math.PI === NaN) {
  throw "expected Math.PI not to be NaN";
}

if (Math.propertyIsEnumerable("PI")) {
  throw "expected Math.PI to be non-enumerable";
}

if (Object.getOwnPropertyDescriptor(Math, "PI").writable) {
  throw "expected Math.PI to be non-writable";
}
