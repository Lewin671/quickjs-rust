// Derived from: test/built-ins/Map/iterator-items-are-not-object.js
var caught = false;
try {
  new Map([1]);
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught) {
  throw new Error("Map iterable constructor must reject non-object entries");
}
