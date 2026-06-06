// Derived from: test/built-ins/RegExp/prototype/Symbol.split/str-limit-capturing.js
var result = /c(d)(e)/[Symbol.split]("abcdefg", 2);

if (!Array.isArray(result)) {
  throw "RegExp.prototype[Symbol.split] should return an array";
}

if (result.length !== 2 || result[0] !== "ab" || result[1] !== "d") {
  throw "RegExp.prototype[Symbol.split] should include captures before applying limit";
}
