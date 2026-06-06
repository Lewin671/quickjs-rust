// Derived from: test/built-ins/Object/getOwnPropertyNames/order-after-define-property.js
var obj = {};
Object.defineProperty(obj, "a", {
  get: function() {},
  set: function(_value) {},
  enumerable: true,
  configurable: true,
});
obj.b = 2;
Object.defineProperty(obj, "a", {
  set: function(_value) {},
});

if (Object.getOwnPropertyNames(obj).join() !== "a,b") {
  throw "expected object property names in creation order";
}

var arr = [];
arr.a = 1;
Object.defineProperty(arr, "length", { value: 2 });

if (Object.getOwnPropertyNames(arr).join() !== "length,a") {
  throw "expected array length before later string properties";
}
