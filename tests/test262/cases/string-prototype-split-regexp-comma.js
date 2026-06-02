// Derived from: test/built-ins/String/prototype/split/separator-regexp-comma-instance-is-string-one-1-two-2-four-4.js
var result = "one-1,two-2,four-4".split(/,/);

if (result.length !== 3 || result[0] !== "one-1" || result[1] !== "two-2" || result[2] !== "four-4") {
  throw "String.prototype.split should parse and use comma RegExp separators";
}
