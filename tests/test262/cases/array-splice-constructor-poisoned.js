// Derived from: test/built-ins/Array/prototype/splice/create-ctor-poisoned.js
var marker = { ok: true };
var array = [];
Object.defineProperty(array, "constructor", {
  get: function() {
    throw marker;
  }
});

var caught = false;
try {
  array.splice();
} catch (error) {
  caught = error === marker;
}
if (!caught) {
  throw "Array.prototype.splice should propagate constructor getter abrupt completions";
}
