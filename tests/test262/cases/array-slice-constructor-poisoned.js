// Derived from: test/built-ins/Array/prototype/slice/create-ctor-poisoned.js
var marker = {};
var a = [];
Object.defineProperty(a, "constructor", {
  get: function() {
    throw marker;
  }
});
var caught = false;
try {
  a.slice();
} catch (error) {
  caught = error === marker;
}
if (!caught) { throw; }
