// Derived from: test/built-ins/Array/prototype/toSorted/comparefn-default.js
var result = [333, 33, 3, 222, 22, 2, 111, 11, 1].toSorted();
if (result.join() !== "1,11,111,2,22,222,3,33,333") {
  throw "Array.prototype.toSorted should sort by string order by default";
}
