// Derived from: test/built-ins/Array/prototype/at/returns-item-relative-index.js
var array = [1, 2, 3, 4, 5];

if (array.at(-1) !== 5) {
  throw "expected at(-1) to return the last item";
}
if (array.at(-3) !== 3) {
  throw "expected at(-3) to return the third item";
}
if (array.at(-5) !== 1) {
  throw "expected at(-5) to return the first item";
}
