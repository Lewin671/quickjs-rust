// Derived from: test/built-ins/Array/prototype/unshift/S15.4.4.13_A1_T1.js
let array = [3];
if (array.unshift(1, 2) !== 3) {
  throw "expected unshift to return the new length";
}
if (array.length !== 3) {
  throw "expected unshift to increase length";
}
if (array.join() !== "1,2,3") {
  throw "expected unshift to prepend values";
}
