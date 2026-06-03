// Derived from: test/built-ins/String/prototype/toLowerCase/special_casing_conditional.js
if ("\uD835\uDCA2\u03A3".toLowerCase() !== "\uD835\uDCA2\u03C2") {
  throw "expected supplementary cased code point before sigma to select final sigma";
}
if ("A\u03A3\uD835\uDCA2".toLowerCase() !== "a\u03C3\uD835\uDCA2") {
  throw "expected supplementary cased code point after sigma to block final sigma";
}
