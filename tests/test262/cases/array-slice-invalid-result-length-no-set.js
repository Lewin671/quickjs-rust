// Derived from: test/built-ins/Array/prototype/slice/create-non-array-invalid-len.js
var callCount = 0;
var object = Object.defineProperty({}, "length", {
  get: function() {
    return 4294967296;
  },
  set: function() {
    callCount = callCount + 1;
  }
});
var caught = false;
try {
  Array.prototype.slice.call(object);
} catch (error) {
  caught = error instanceof RangeError;
}
if (!caught) { throw; }
if (callCount !== 0) { throw; }
