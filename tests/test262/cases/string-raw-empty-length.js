// Derived from: test/built-ins/String/raw/return-empty-string-if-length-is-zero-or-less-number.js
if (String.raw({ raw: { length: 0 } }) !== "") {
  throw new Error("zero raw length must return empty string");
}
if (String.raw({ raw: { 0: "a", length: -1 } }, "b") !== "") {
  throw new Error("negative raw length must return empty string");
}
