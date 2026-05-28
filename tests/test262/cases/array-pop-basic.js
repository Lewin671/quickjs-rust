// Derived from: test/built-ins/Array/prototype/pop/S15.4.4.6_A1.1_T1.js
let array = [1, 2, 3];
if (array.pop() !== 3) {
  throw "expected pop to return the last element";
}
if (array.length !== 2) {
  throw "expected pop to decrease length";
}
if (array.join() !== "1,2") {
  throw "expected pop to remove the last element";
}
if ([].pop() !== undefined) {
  throw "expected pop on an empty array to return undefined";
}
