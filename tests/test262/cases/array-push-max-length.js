// Derived from: test/built-ins/Array/prototype/push/S15.4.4.7_A3.js
var array = [];
array.length = 4294967295;
if (array.push() !== 4294967295) {
  throw "expected empty push to return max array length";
}
var caught = false;
try {
  array.push("x");
} catch (error) {
  caught = error instanceof RangeError;
}
if (!caught) {
  throw "expected push beyond max array length to throw RangeError";
}
if (array[4294967295] !== "x") {
  throw "expected push to set non-index property before length failure";
}
if (array.length !== 4294967295) {
  throw "expected push length failure to preserve array length";
}
