// Derived from: test/built-ins/Set/prototype/union/combines-sets.js

var result = new Set([1, 2]).union(new Set([2, 3]));
var seen = "";
result.forEach(function(value) { seen = seen + value; });
if (!(result instanceof Set)) {
  throw "union should return a Set";
}
if (result.size !== 3 || seen !== "123") {
  throw "union should combine set values in order";
}
