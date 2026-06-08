// Derived from: test/annexB/built-ins/RegExp/RegExp-decimal-escape-class-range.js
var result = /[\d][\12-\14]{1,}[^\d]/.exec("line1\n\n\n\n\nline2");

if (result.length !== 1) {
  throw "RegExp decimal escape class range should produce one match";
}

if (result.index !== 4) {
  throw "RegExp decimal escape class range should report the match index";
}

if (result.input !== "line1\n\n\n\n\nline2") {
  throw "RegExp decimal escape class range should preserve the input";
}

if (result[0] !== "1\n\n\n\n\nl") {
  throw "RegExp decimal escape class range should match legacy octal code units";
}
