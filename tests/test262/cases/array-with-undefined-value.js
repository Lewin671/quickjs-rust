// Derived from: test/built-ins/Array/prototype/with/index-casted-to-number.js
var result = [1, 2, 3].with(1);
if (result.join("|") !== "1||3" || result[1] !== undefined) {
  throw "Array.prototype.with should use undefined when value is omitted";
}
