// Derived from: test/built-ins/String/prototype/split/argument-is-regexp-reg-exp-d-and-instance-is-string-dfe23iu-34-65.js
var result = "dfe23iu 34 =+65--".split(new RegExp("\\d+"));

if (result.length !== 4 || result[0] !== "dfe" || result[1] !== "iu " || result[2] !== " =+" || result[3] !== "--") {
  throw "String.prototype.split should use greedy RegExp separator matches";
}
