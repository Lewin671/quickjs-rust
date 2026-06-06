// Derived from: test/built-ins/Symbol/is-constructor.js
var constructed = Reflect.construct(function () {
  this.marker = 1;
}, [], Symbol);

if (constructed.marker !== 1) {
  throw new Error("expected Symbol to be accepted as newTarget");
}

try {
  new Symbol("test");
  throw new Error("expected new Symbol to throw");
} catch (error) {
  if (!(error instanceof TypeError)) {
    throw error;
  }
}
