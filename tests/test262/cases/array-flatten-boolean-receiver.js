// Derived from: test/built-ins/Array/prototype/flat/call-with-boolean.js
// Derived from: test/built-ins/Array/prototype/flatMap/call-with-boolean.js

var trueFlat = Array.prototype.flat.call(true);
if (trueFlat.length !== 0) {
  throw "Array.prototype.flat should return an empty array for true receiver";
}

var falseFlat = Array.prototype.flat.call(false);
if (falseFlat.length !== 0) {
  throw "Array.prototype.flat should return an empty array for false receiver";
}

var called = 0;
var trueFlatMap = Array.prototype.flatMap.call(true, function() {
  called = called + 1;
});
if (trueFlatMap.length !== 0 || called !== 0) {
  throw "Array.prototype.flatMap should not call mapper for true receiver with length zero";
}

var falseFlatMap = Array.prototype.flatMap.call(false, function() {
  called = called + 1;
});
if (falseFlatMap.length !== 0 || called !== 0) {
  throw "Array.prototype.flatMap should not call mapper for false receiver with length zero";
}
