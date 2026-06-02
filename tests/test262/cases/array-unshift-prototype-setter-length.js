// Derived from: test/built-ins/Array/prototype/unshift/set-length-array-length-is-non-writable.js
var array = [];
var calls = 0;
Object.defineProperty(Array.prototype, "0", {
  set: function(_value) {
    Object.defineProperty(array, "length", { writable: false });
    calls++;
  },
  configurable: true
});

var caught = false;
try {
  array.unshift(1);
} catch (error) {
  caught = error instanceof TypeError;
}
delete Array.prototype[0];

if (!caught) {
  throw "expected unshift to throw when final length set fails";
}
if (calls !== 1 || array.hasOwnProperty("0") || array.length !== 0) {
  throw "expected prototype setter side effect without own element insertion";
}
