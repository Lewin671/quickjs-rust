// Derived from: test/built-ins/Object/entries/primitive-strings.js
var entries = Object.entries("ab");
if (entries.length !== 2 ||
    entries[0][0] !== "0" || entries[0][1] !== "a" ||
    entries[1][0] !== "1" || entries[1][1] !== "b") {
  throw "Object.entries should expose string index entries";
}
