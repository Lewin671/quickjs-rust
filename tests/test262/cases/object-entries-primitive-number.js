// Derived from: test/built-ins/Object/entries/primitive-numbers.js
if (Object.entries(0).length !== 0) {
  throw "Object.entries should return an empty array for primitive numbers";
}
