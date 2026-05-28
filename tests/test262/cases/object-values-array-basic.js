// Derived from: test/built-ins/Object/values/return-order.js
var values = Object.values([4, 5]);
if (values.length !== 2 || values[0] !== 4 || values[1] !== 5) {
  throw "Object.values should return array element values";
}
