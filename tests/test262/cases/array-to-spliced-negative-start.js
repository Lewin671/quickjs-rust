// Derived from: test/built-ins/Array/prototype/toSpliced/start-neg-subtracted-from-length.js
if ([1, 2, 3].toSpliced(-1, 1, 9).join() !== "1,2,9") {
  throw "Array.prototype.toSpliced should resolve negative start from the end";
}
