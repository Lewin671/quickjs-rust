// Derived from: test/built-ins/Reflect/ownKeys/order-after-define-property.js
var obj = {};
var symA = Symbol("a");
var symB = Symbol("b");
obj[symA] = 1;
obj[symB] = 2;
Object.defineProperty(obj, symA, { configurable: false });

var keys = Reflect.ownKeys(obj);
if (keys.length !== 2) {
  throw "expected two symbol keys";
}
if (keys[0] !== symA || keys[1] !== symB) {
  throw "expected symbol own keys";
}
