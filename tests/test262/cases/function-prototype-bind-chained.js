// Derived from: test/built-ins/Function/prototype/bind/15.3.4.5.1-4-3.js
function add(a, b) { return a + b; }
var bound = add.bind(null, 2).bind({ ignored: true }, 3);
if (bound() !== 5) {
  throw "bound function should preserve previously bound arguments";
}
