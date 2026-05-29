// Derived from: test/built-ins/String/prototype/trimStart/prop-desc.js
if ("  abc  ".trimLeft() !== "abc  ") {
  throw "trimLeft should trim leading whitespace";
}

if (String.prototype.trimLeft !== String.prototype.trimStart) {
  throw "trimLeft should alias trimStart";
}

if (String.prototype.trimLeft.length !== 0) {
  throw "trimLeft length should match trimStart";
}
