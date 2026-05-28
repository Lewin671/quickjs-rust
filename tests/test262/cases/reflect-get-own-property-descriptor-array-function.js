// Derived from: test/built-ins/Reflect/getOwnPropertyDescriptor/getOwnPropertyDescriptor.js
if (Reflect.getOwnPropertyDescriptor([1, 2], "length").enumerable !== false) {
  throw "expected array length descriptor";
}
function fn(a, b) {}
if (Reflect.getOwnPropertyDescriptor(fn, "length").value !== 2) {
  throw "expected function length descriptor";
}
