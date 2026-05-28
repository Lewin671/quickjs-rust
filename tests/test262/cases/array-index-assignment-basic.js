// Derived from: test/built-ins/Array/prototype/push/S15.4.4.7_A1_T1.js
let array = [1];
array[2] = 3;
if (array.length !== 3) {
  throw "expected index assignment past the end to extend length";
}
if (array.join() !== "1,,3") {
  throw "expected skipped indexes to read as undefined";
}
array.length = 1;
if (array.join() !== "1") {
  throw "expected length assignment to truncate array contents";
}
