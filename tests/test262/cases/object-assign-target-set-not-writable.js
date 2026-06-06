// Derived from: test/built-ins/Object/assign/target-set-not-writable.js
var target = {};
Object.defineProperty(target, "attr", {
  writable: false
});

var caught = false;
try {
  Object.assign(target, { attr: 1 });
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught) {
  throw "expected Object.assign to throw when setting a non-writable property";
}
