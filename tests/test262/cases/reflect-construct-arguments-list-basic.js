// Derived from: test/built-ins/Reflect/construct/use-arguments-list.js
function C() {
  this.count = arguments.length;
  this.first = arguments[0];
  this.second = arguments[1];
}

var result = Reflect.construct(C, { 0: 42, 1: "value", length: 2 });

if (result.count !== 2) {
  throw "Reflect.construct should read array-like argument list length";
}
if (result.first !== 42 || result.second !== "value") {
  throw "Reflect.construct should read array-like argument list elements";
}
