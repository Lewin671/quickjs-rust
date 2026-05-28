// Derived from: test/built-ins/Object/entries/inherited-properties-omitted.js
var object = Object.create({ inherited: 1 }, { own: { value: 2, enumerable: true } });
var entries = Object.entries(object);
if (entries.length !== 1 || entries[0][0] !== "own" || entries[0][1] !== 2) {
  throw "Object.entries should omit inherited properties";
}
