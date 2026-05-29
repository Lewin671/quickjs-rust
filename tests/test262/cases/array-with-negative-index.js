// Derived from: test/built-ins/Array/prototype/with/index-negative.js
if ([1, 2, 3].with(-1, 9).join() !== "1,2,9") {
  throw "Array.prototype.with should resolve negative indexes from the end";
}
