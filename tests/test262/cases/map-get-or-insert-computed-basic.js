// Derived from: test/built-ins/Map/prototype/getOrInsertComputed/append-new-values.js

var map = new Map();
var calls = 0;

if (map.getOrInsertComputed("a", function(key) {
  calls = calls + 1;
  return key + "!";
}) !== "a!") {
  throw "getOrInsertComputed should return the computed value";
}
if (map.get("a") !== "a!") {
  throw "getOrInsertComputed should store the computed value";
}
if (map.getOrInsertComputed("a", function() {
  calls = calls + 1;
  return "wrong";
}) !== "a!") {
  throw "getOrInsertComputed should return existing values";
}
if (calls !== 1) {
  throw "getOrInsertComputed should not call callback for existing keys";
}
map.getOrInsertComputed("b", function(key) {
  map.set(key, "inner");
  return "outer";
});
if (map.get("b") !== "outer") {
  throw "getOrInsertComputed should overwrite callback mutations for the key";
}
if (Map.prototype.getOrInsertComputed.length !== 2) {
  throw "getOrInsertComputed length should be 2";
}
