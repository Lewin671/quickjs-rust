// Derived from: test/built-ins/Set/constructor.js
if (typeof Set !== "function") {
  throw "Set should be a function";
}
if (Set.length !== 0) {
  throw "Set.length should be 0";
}
var set = new Set();
if (!(set instanceof Set)) {
  throw "new Set should create Set instances";
}
if (Object.prototype.toString.call(set) !== "[object Set]") {
  throw "Set instances should have the Set toString tag";
}
