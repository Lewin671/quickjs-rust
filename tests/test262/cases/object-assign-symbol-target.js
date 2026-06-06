// Derived from: test/built-ins/Object/assign/Target-Symbol.js
var target = Symbol("foo");
var result = Object.assign(target, {
  a: 1
});
if (typeof result !== "object") { throw; }
if (result.toString() !== "Symbol(foo)") { throw; }
if (result.a !== 1) { throw; }
