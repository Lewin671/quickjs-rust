// Derived from: test/built-ins/String/raw/return-the-string-value.js
if (String.raw({ raw: ["a", "b", "c"] }, 1, 2) !== "a1b2c") {
  throw new Error("String.raw must interleave raw segments and substitutions");
}
