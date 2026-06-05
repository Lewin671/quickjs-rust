// Derived from: test/built-ins/Set/prototype/symmetricDifference/combines-sets.js

var result = new Set([1, 2]).symmetricDifference(new Set([2, 3]));
var seen = "";
result.forEach(function(value) { seen = seen + value; });
if (result.size !== 2 || seen !== "13") {
  throw "symmetricDifference should keep values from either set but not both";
}
