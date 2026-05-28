// Derived from: test/built-ins/Function/prototype/bind/15.3.4.5.1-4-13.js
function add(a, b) { return this.base + a + b; }
var context = { base: 4 };
var bound = add.bind(context, 2);
if (bound(3) !== 9) {
  throw "Function.prototype.bind should bind this and leading arguments";
}
