// Derived from: test/built-ins/RegExp/character-class-escape-non-whitespace.js
var whitespaceChars = [
  "\t",
  "\n",
  "\v",
  "\f",
  "\r",
  " ",
  "\u00a0",
  "\u1680",
  "\u2000",
  "\u200a",
  "\u2028",
  "\u2029",
  "\u202f",
  "\u205f",
  "\u3000",
  "\ufeff",
];

for (var i = 0; i < whitespaceChars.length; i++) {
  var whitespace = whitespaceChars[i];
  if (!/\s/.test(whitespace) || !/[\s]/.test(whitespace)) {
    throw new Error("expected whitespace escape match for " + i);
  }
  if (/\S/.test(whitespace) || /[\S]/.test(whitespace)) {
    throw new Error("expected non-whitespace escape miss for " + i);
  }
}

var nonWhitespaceChars = ["\u0085", "\u180e", "A", "_", "0"];

for (var j = 0; j < nonWhitespaceChars.length; j++) {
  var nonWhitespace = nonWhitespaceChars[j];
  if (/\s/.test(nonWhitespace) || /[\s]/.test(nonWhitespace)) {
    throw new Error("expected whitespace escape miss for " + j);
  }
  if (!/\S/.test(nonWhitespace) || !/[\S]/.test(nonWhitespace)) {
    throw new Error("expected non-whitespace escape match for " + j);
  }
}
