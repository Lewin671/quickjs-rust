// Derived from: test/built-ins/String/prototype/padStart/normal-operation.js
if ("abc".padStart(7, "def") !== "defdabc") {
  throw "expected padStart to truncate repeated fill string";
}
if ("abc".padStart(5, "*") !== "**abc") {
  throw "expected padStart to prepend repeated fill characters";
}
