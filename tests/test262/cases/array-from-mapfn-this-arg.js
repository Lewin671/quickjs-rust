// Derived from: test/built-ins/Array/from/iter-map-fn-this-arg.js
var receiver = { offset: 4 };
var result = Array.from([1], function(value) {
  return value + this.offset;
}, receiver);
if (result[0] !== 5) {
  throw "Array.from should pass thisArg to mapfn";
}
