// Derived from: test/built-ins/Array/prototype/flat/null-undefined-elements.js
var actual = [1, [null, undefined]].flat();
if (actual.length !== 3) {
  throw "Array.prototype.flat should preserve null and undefined elements";
}
if (actual[1] !== null || actual[2] !== undefined) {
  throw "Array.prototype.flat should keep null and undefined values";
}
