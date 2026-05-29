// Derived from: test/built-ins/Array/prototype/toSpliced/start-bigger-than-length.js
if ([1, 2, 3].toSpliced(8, 1, 4).join() !== "1,2,3,4") {
  throw "Array.prototype.toSpliced should clamp start past length to length";
}
