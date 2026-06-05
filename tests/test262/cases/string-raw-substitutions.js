// Derived from: test/built-ins/String/raw/substitutions-are-appended-on-same-index.js
var actual = String.raw({ raw: { 0: "x", 1: "y", 2: "z", length: 3 } }, "A");
if (actual !== "xAyz") {
  throw new Error("missing substitutions must contribute an empty string");
}
