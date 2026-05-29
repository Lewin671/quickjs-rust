// Derived from: test/built-ins/Reflect/apply/return-target-call-result.js
var object = {};

function fn() {
  return object;
}

if (Reflect.apply(fn, 1, []) !== object) {
  throw "expected Reflect.apply to return target call result";
}
