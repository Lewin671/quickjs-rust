// Derived from: test/built-ins/String/raw/substitutions-are-limited-to-template-raw-length.js
var unused = {
  toString: function() {
    throw "unused substitution should not be coerced";
  }
};

if (String.raw({ raw: ["a", "c", "e"] }, "b", "d", unused) !== "abcde") {
  throw "expected substitutions to be limited by raw length";
}
