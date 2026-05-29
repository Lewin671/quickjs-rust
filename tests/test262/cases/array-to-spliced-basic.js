// Derived from: test/built-ins/Array/prototype/toSpliced/immutable.js
var source = [1, 2, 3, 4];
var result = source.toSpliced(1, 2, "a", "b");
if (result.join() !== "1,a,b,4") {
  throw "Array.prototype.toSpliced should return a spliced copy";
}
if (source.join() !== "1,2,3,4" || result === source) {
  throw "Array.prototype.toSpliced should not mutate the receiver";
}
