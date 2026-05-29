// Derived from: test/built-ins/Reflect/get/return-value.js
function fn(a, b) {}
fn.value = 3;
var array = [5, 7];
if (Reflect.get(fn, "length") !== 2 || Reflect.get(fn, "value") !== 3) {
  throw "expected Reflect.get to read function properties";
}
if (Reflect.get(array, "1") !== 7 || Reflect.get(array, "length") !== 2) {
  throw "expected Reflect.get to read array index and length properties";
}
