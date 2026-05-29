// Derived from: test/built-ins/String/prototype/trimEnd/prop-desc.js
if ("  abc  ".trimRight() !== "  abc") {
  throw "trimRight should trim trailing whitespace";
}

if (String.prototype.trimRight !== String.prototype.trimEnd) {
  throw "trimRight should alias trimEnd";
}

if (String.prototype.trimRight.length !== 0) {
  throw "trimRight length should match trimEnd";
}
