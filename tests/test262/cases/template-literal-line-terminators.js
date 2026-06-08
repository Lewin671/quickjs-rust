// Derived from: test/language/expressions/template-literal/tv-line-continuation.js
(function(s) {
  if (s[0] !== "") { throw new Error("line continuation cooked value should be empty"); }
  if (s.raw[0] !== "\\\n") { throw new Error("line continuation raw value should preserve the slash and LF"); }
})`\
`;

// Derived from: test/language/expressions/template-literal/tv-line-terminator-sequence.js
(function(s) {
  if (s[0] !== "\u2028\u2029") { throw new Error("line separator cooked values should be preserved"); }
  if (s.raw[0] !== "\u2028\u2029") { throw new Error("line separator raw values should be preserved"); }
})`  `;
