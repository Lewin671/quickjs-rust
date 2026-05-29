// Derived from: test/built-ins/Array/prototype/toSpliced/deleteCount-undefined.js
if ([1, 2, 3].toSpliced(1, undefined, 9).join() !== "1,9,2,3") {
  throw "Array.prototype.toSpliced should treat undefined deleteCount as zero";
}
