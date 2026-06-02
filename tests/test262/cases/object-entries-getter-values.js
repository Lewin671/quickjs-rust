// Derived from: test/built-ins/Object/entries/getter-adding-key.js
var object = {
  a: "A",
  get b() {
    return "B";
  }
};
var entries = Object.entries(object);
if (entries.length !== 2) {
  throw "expected Object.entries to include enumerable getter key";
}
if (entries[0][0] !== "a" || entries[0][1] !== "A") {
  throw "expected first Object.entries pair";
}
if (entries[1][0] !== "b" || entries[1][1] !== "B") {
  throw "expected Object.entries to invoke getter";
}
