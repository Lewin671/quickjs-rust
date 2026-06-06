// Derived from: test/built-ins/Object/assign/target-set-user-error.js
var sentinel = {};
var target = {};
Object.defineProperty(target, "attr", {
  set: function(_) {
    throw sentinel;
  }
});

var caught = false;
try {
  Object.assign(target, { attr: 1 });
} catch (error) {
  caught = error === sentinel;
}
if (!caught) {
  throw "expected Object.assign to propagate target setter errors";
}
