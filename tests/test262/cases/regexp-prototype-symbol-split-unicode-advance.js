// Derived from: test/built-ins/RegExp/prototype/Symbol.split/u-lastindex-adv-thru-match.js
var result = /./u[Symbol.split]("\uD834\uDF06");

if (result.length !== 2 || result[0] !== "" || result[1] !== "") {
  throw "RegExp.prototype[Symbol.split] should advance over surrogate pairs in unicode mode";
}
