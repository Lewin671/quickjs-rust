// Derived from: test/built-ins/Array/prototype/at/returns-undefined-for-out-of-range-index.js
var array = [1, 2, 3];

if (array.at(3) !== undefined) {
  throw "expected positive out-of-range index to return undefined";
}
if (array.at(-4) !== undefined) {
  throw "expected negative out-of-range index to return undefined";
}
