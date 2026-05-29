// Derived from: test/built-ins/Array/prototype/with/immutable.js
var source = [1, 2, 3];
var result = source.with(1, 9);
if (result.join() !== "1,9,3") {
  throw "Array.prototype.with should replace the indexed element";
}
if (source.join() !== "1,2,3" || result === source) {
  throw "Array.prototype.with should not mutate the receiver";
}
