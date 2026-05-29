// Derived from: test/built-ins/Reflect/apply/call-target.js
var context = { base: 4 };
var count = 0;
var observedThis;
var observedLength;

function fn(a, b, c) {
  count++;
  observedThis = this;
  observedLength = arguments.length;
  return this.base + a + b + (c === undefined ? 0 : c);
}

var result = Reflect.apply(fn, context, [2, 3, undefined]);
if (count !== 1) {
  throw "expected Reflect.apply to call target once";
}
if (observedThis !== context) {
  throw "expected Reflect.apply to pass thisArgument";
}
if (observedLength !== 3) {
  throw "expected Reflect.apply to pass array arguments";
}
if (result !== 9) {
  throw "expected Reflect.apply to return target result";
}
