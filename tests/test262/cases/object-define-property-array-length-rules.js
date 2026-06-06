// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-117.js
// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-125.js
// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-133.js
// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-134.js
// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-136.js
// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-188.js
// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-189.js
function assertThrowsRangeError(value) {
  let array = [];
  let caught = false;
  try {
    Object.defineProperty(array, "length", { value });
  } catch (error) {
    caught = error instanceof RangeError;
  }
  if (!caught || array.length !== 0) {
    throw "expected invalid array length to throw RangeError";
  }
}

assertThrowsRangeError(undefined);
assertThrowsRangeError(-9);
assertThrowsRangeError(Infinity);
assertThrowsRangeError(NaN);

let array = [0, 1, 2];
Object.defineProperty(array, "2", { configurable: false });
let caught = false;
try {
  Object.defineProperty(array, "length", { value: 1 });
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught || array.length !== 3 || array[2] !== 2) {
  throw "expected non-configurable array element to block length shrink";
}

array = [1, 2, 3];
Object.defineProperty(array, "length", { writable: false });
caught = false;
try {
  Object.defineProperty(array, "3", { value: "abc" });
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught || array.length !== 3 || array.hasOwnProperty("3")) {
  throw "expected non-writable length to reject index equal to length";
}

caught = false;
try {
  Object.defineProperty(array, "4", { value: "abc" });
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught || array.length !== 3 || array.hasOwnProperty("4")) {
  throw "expected non-writable length to reject index greater than length";
}
