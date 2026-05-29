// Derived from: test/built-ins/Array/prototype/sort/S15.4.4.11_A1.1_T1.js
var array = [3, 20, 100, 1];
var result = array.sort();
if (result !== array) {
  throw "Array.prototype.sort should return the receiver";
}
if (array.join() !== "1,100,20,3") {
  throw "Array.prototype.sort should use string order when comparefn is omitted";
}
