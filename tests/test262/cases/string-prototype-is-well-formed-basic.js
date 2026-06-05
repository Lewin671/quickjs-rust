// Derived from: test/built-ins/String/prototype/isWellFormed/returns-boolean.js
if (typeof String.prototype.isWellFormed !== "function") {
  throw new Error("isWellFormed must be a function");
}
if (!"abc".isWellFormed()) {
  throw new Error("plain strings must be well-formed");
}
if (!"\uD83D\uDCA9".isWellFormed()) {
  throw new Error("surrogate pairs must be well-formed");
}
if ("\uD83D".isWellFormed()) {
  throw new Error("lone leading surrogate must not be well-formed");
}
if ("\uDCA9".isWellFormed()) {
  throw new Error("lone trailing surrogate must not be well-formed");
}
