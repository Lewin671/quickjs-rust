// Derived from: test/built-ins/String/prototype/padEnd/normal-operation.js
if ("abc".padEnd(7, "def") !== "abcdefd") {
  throw "expected padEnd to truncate repeated fill string";
}
if ("abc".padEnd(5, "*") !== "abc**") {
  throw "expected padEnd to append repeated fill characters";
}
