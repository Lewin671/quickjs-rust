// Derived from: test/built-ins/Object/assign/target-is-sealed-existing-accessor-property.js
var seen = 1;
var target = Object.seal({
  set value(next) {
    seen = next;
  }
});

Object.assign(target, { value: 2 });
if (seen !== 2) {
  throw "expected Object.assign to call setter on sealed target";
}
