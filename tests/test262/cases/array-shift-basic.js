// Derived from: test/built-ins/Array/prototype/shift/S15.4.4.9_A1.1_T1.js
let array = [1, 2, 3];
if (array.shift() !== 1) {
  throw "expected shift to return the first element";
}
if (array.length !== 2) {
  throw "expected shift to decrease length";
}
if (array.join() !== "2,3") {
  throw "expected shift to move remaining elements left";
}
if ([].shift() !== undefined) {
  throw "expected shift on an empty array to return undefined";
}
