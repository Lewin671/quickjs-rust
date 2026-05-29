// Derived from: test/built-ins/Array/prototype/splice/S15.4.4.12_A2.1_T1.js
var array = [1, 4];
var removed = array.splice(1, 0, 2, 3);
if (removed.length !== 0) {
  throw "Array.prototype.splice should return an empty array when deleteCount is zero";
}
if (array.join() !== "1,2,3,4") {
  throw "Array.prototype.splice should insert items at start";
}
