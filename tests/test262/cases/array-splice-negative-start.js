// Derived from: test/built-ins/Array/prototype/splice/S15.4.4.12_A1.2_T1.js
var array = [1, 2, 3, 4];
var removed = array.splice(-2, 1);
if (removed.join() !== "3") {
  throw "Array.prototype.splice should resolve negative start from the end";
}
if (array.join() !== "1,2,4") {
  throw "Array.prototype.splice should delete from resolved negative start";
}
