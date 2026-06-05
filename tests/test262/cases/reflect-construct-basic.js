// Derived from: test/built-ins/Reflect/construct/construct.js
function C(a, b) {
  this.sum = a + b;
}

var result = Reflect.construct(C, [2, 5]);

if (!(result instanceof C)) {
  throw "Reflect.construct should create instances of target";
}
if (result.sum !== 7) {
  throw "Reflect.construct should pass arguments to target";
}
