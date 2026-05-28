// Derived from: test/built-ins/Reflect/has/has.js
var arrayProto = { marker: 11 };
var array = [];
Object.setPrototypeOf(array, arrayProto);
function fn() {}
if (Reflect.has(array, "length") !== true) {
  throw "expected array length";
}
if (Reflect.has(array, "marker") !== true) {
  throw "expected array inherited property";
}
if (Reflect.has(fn, "call") !== true) {
  throw "expected function inherited property";
}
