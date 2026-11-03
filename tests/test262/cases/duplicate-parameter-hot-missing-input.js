// Derived from: test/language/function-code/S10.2.1_A3.js
function last(a, a) { return a; }
function call(x) { return last(x); }
var object = { method: last };
for (var i = 0; i < 100; i++) {
  if (call(i) !== undefined) throw 'missing last argument in hot call';
  if (object.method(i) !== undefined) throw 'missing last argument in method';
  if (last(1, 2) !== 2) throw 'last supplied argument';
}
