// Derived from: test/built-ins/String/prototype/split/arguments-are-regexp-l-and-2-and-instance-is-string-hello.js
var result = "hello".split(/l/, 2);

if (result.length !== 2 || result[0] !== "he" || result[1] !== "") {
  throw "String.prototype.split should apply limit to RegExp separator results";
}
