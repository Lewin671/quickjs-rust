// Derived from: test/built-ins/Array/prototype/splice/create-non-array-invalid-len.js
var setterCalls = 0;
var object = Object.defineProperty({}, "length", {
  get: function() {
    return Math.pow(2, 32);
  },
  set: function() {
    setterCalls += 1;
  }
});

var caught = false;
try {
  Array.prototype.splice.call(object, 0);
} catch (error) {
  caught = error instanceof RangeError;
}
if (!caught || setterCalls !== 0) {
  throw "Array.prototype.splice should reject an oversized result array before mutating length";
}
