// Derived from: test/built-ins/Set/prototype/values/values.js
var set = new Set();
set.add("a");
set.add("b");
var values = set.values();
var first = values.next();
var second = values.next();
var last = values.next();
if (first.done || first.value !== "a") {
  throw "Set.prototype.values should yield values";
}
if (second.done || second.value !== "b") {
  throw "Set.prototype.values should preserve insertion order";
}
if (!last.done || last.value !== undefined) {
  throw "Set iterator should finish with undefined value";
}
var entry = set.entries().next().value;
if (entry[0] !== "a" || entry[1] !== "a") {
  throw "Set.prototype.entries should yield value-value pairs";
}
if (set.keys().next().value !== "a") {
  throw "Set.prototype.keys should alias values behavior";
}
