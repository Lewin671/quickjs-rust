// Derived from: test/built-ins/Array/prototype/splice/S15.4.4.12_A1.1_T1.js
var array = [1, 2, 3, 4];
var removed = array.splice(1, 2);
if (removed.join() !== "2,3") {
  throw "Array.prototype.splice should return deleted elements";
}
if (array.join() !== "1,4") {
  throw "Array.prototype.splice should remove deleted elements from receiver";
}
