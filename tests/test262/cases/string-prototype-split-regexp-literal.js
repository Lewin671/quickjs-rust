// Derived from: test/built-ins/String/prototype/split/argument-is-regexp-l-and-instance-is-string-hello.js
var result = "hello".split(/l/);

if (result.length !== 3 || result[0] !== "he" || result[1] !== "" || result[2] !== "o") {
  throw "String.prototype.split should accept RegExp separators";
}
