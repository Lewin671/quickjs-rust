// Derived from: test/built-ins/Set/prototype/difference/combines-sets.js

var result = new Set([1, 2]).difference(new Set([2, 3]));
var seen = "";
result.forEach(function(value) { seen = seen + value; });
if (result.size !== 1 || seen !== "1") {
  throw "difference should keep left-only values";
}
