(() => {
  const p = [Boolean.prototype, Number.prototype, String.prototype, BigInt.prototype, Symbol.prototype];
  const values = [true, 1, 'x', 2n, Symbol()];
  function f() { return Object.getPrototypeOf(this); }
  const nested = Function('return function inner(value) { return value + 1; };')();
  Boolean = Number = String = BigInt = Symbol = function Replacement() {};
  return values.map((value, i) => f.call(value) === p[i]).join(':') + '|' + nested.toString();
})()
