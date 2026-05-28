// Derived from: test/built-ins/Array/prototype/push/S15.4.4.7_A1_T1.js
let array = [1];
if (array.push(2, 3) !== 3) {
  throw "expected push to return the new length";
}
if (array.length !== 3) {
  throw "expected push to increase length";
}
if (array.join() !== "1,2,3") {
  throw "expected push to append values";
}
