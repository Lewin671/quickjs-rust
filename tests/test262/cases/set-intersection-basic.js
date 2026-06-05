// Derived from: test/built-ins/Set/prototype/intersection/combines-sets.js

var result = new Set([1, 2]).intersection(new Set([2, 3]));
var seen = "";
result.forEach(function(value) { seen = seen + value; });
if (result.size !== 1 || seen !== "2") {
  throw "intersection should keep shared values";
}
