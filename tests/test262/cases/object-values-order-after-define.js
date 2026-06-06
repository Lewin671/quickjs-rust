// Derived from: test/built-ins/Object/values/order-after-define-property.js
var obj = {};
Object.defineProperty(obj, "a", {
  get: function() {},
  enumerable: true,
  configurable: true,
});
obj.b = "b";
Object.defineProperty(obj, "a", {
  get: function() {
    return "a";
  },
});
var values = Object.values(obj);
if (values.length !== 2) { throw; }
if (values[0] !== "a") { throw; }
if (values[1] !== "b") { throw; }
