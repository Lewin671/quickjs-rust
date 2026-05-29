// Derived from: test/built-ins/Array/prototype/toSpliced/deleteCount-missing.js
if ([1, 2, 3].toSpliced(1).join() !== "1") {
  throw "Array.prototype.toSpliced should delete through the end when deleteCount is missing";
}
