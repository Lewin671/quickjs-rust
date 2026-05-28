// Derived from: test/built-ins/Object/values/return-order.js
var values = Object.values({ first: 1, second: 2 });
if (values.length !== 2 || values[0] !== 1 || values[1] !== 2) {
  throw "Object.values should return own enumerable property values";
}
