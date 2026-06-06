// Derived from: test/built-ins/Object/assign/target-is-sealed-existing-accessor-property.js
var symbol = Symbol();
var seen = 1;
var target = {
  set [symbol](value) {
    seen = value;
  }
};

Object.seal(target);
Object.assign(target, { [symbol]: 2 });
if (seen !== 2) {
  throw "expected Object.assign to call symbol setter on sealed target";
}
