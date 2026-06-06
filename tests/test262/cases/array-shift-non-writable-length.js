// Derived from: test/built-ins/Array/prototype/shift/throws-when-this-value-length-is-writable-false.js
let object = Object.defineProperty({}, "length", { writable: false });
let caught = false;
try {
  Array.prototype.shift.call(object);
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught) {
  throw "expected shift to throw when length is non-writable";
}
