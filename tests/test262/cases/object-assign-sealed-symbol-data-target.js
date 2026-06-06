// Derived from: test/built-ins/Object/assign/target-is-sealed-existing-data-property.js
var symbol = Symbol();
var target = { [symbol]: 1 };

Object.seal(target);
Object.assign(target, { [symbol]: 2 });
if (target[symbol] !== 2) {
  throw "expected Object.assign to update symbol data property on sealed target";
}
