// Derived from: test/built-ins/String/prototype/toWellFormed/returns-well-formed-string.js
if (typeof String.prototype.toWellFormed !== "function") {
  throw new Error("toWellFormed must be a function");
}
if ("\uD83D".toWellFormed().charCodeAt(0) !== 0xFFFD) {
  throw new Error("lone leading surrogate must be replaced");
}
if ("\uDCA9".toWellFormed().charCodeAt(0) !== 0xFFFD) {
  throw new Error("lone trailing surrogate must be replaced");
}
var pair = "\uD83D\uDCA9";
if (pair.toWellFormed() !== pair) {
  throw new Error("well-formed surrogate pair must be preserved");
}
