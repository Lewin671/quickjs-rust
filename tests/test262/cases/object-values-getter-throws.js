// Derived from: test/built-ins/Object/values/exception-during-enumeration.js
var caught = false;
try {
  Object.values({
    get a() {
      throw new RangeError("expected");
    }
  });
} catch (error) {
  caught = error instanceof RangeError;
}
if (!caught) {
  throw "expected Object.values to propagate getter errors";
}
