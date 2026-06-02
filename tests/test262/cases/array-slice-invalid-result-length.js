// Derived from: test/built-ins/Array/prototype/slice/S15.4.4.10_A3_T1.js
var object = { length: 4294967296 };
var caught = false;
try {
  Array.prototype.slice.call(object, 0, 4294967296);
} catch (error) {
  caught = error instanceof RangeError;
}
if (!caught) { throw; }
