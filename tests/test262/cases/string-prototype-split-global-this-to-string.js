// Derived from: test/built-ins/String/prototype/split/checking-by-using-eval.js
var split = String.prototype.split.bind(this);
toString = Object.prototype.toString;

var result = split("[", {
  valueOf: function () {
    return 5;
  },
});

if (result.length !== 2 || result[0] !== "" || result[1].substring(0, 6) !== "object") {
  throw "String.prototype.split should use global object string conversion";
}
