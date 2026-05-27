// Derived from: test/built-ins/Array/prototype/at/returns-item.js
var array = [1, 2, 3, 4, 5];

if (array.at(0) !== 1) {
  throw "expected at(0) to return the first item";
}
if (array.at(2) !== 3) {
  throw "expected at(2) to return the third item";
}
if (array.at(4) !== 5) {
  throw "expected at(4) to return the fifth item";
}
