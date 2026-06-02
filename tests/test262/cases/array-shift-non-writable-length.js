// Derived from: test/built-ins/Array/prototype/shift/throws-when-this-value-length-is-writable-false.js
let object = { 0: 1, length: 1 };
Object.defineProperty(object, "length", { value: 1 });
let caught = false;
try {
  Array.prototype.shift.call(object);
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught) {
  throw "expected shift to throw when length is non-writable";
}
