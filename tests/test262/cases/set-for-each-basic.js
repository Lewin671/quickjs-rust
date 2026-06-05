// Derived from: test/built-ins/Set/prototype/forEach/forEach.js
var set = new Set();
set.add("a");
set.add("b");
var seen = "";
var thisArg = { marker: "ctx" };
var returned = set.forEach(function(value, key, receiver) {
  seen += this.marker + ":" + key + ":" + value + ":" + (receiver === set) + "|";
}, thisArg);
if (seen !== "ctx:a:a:true|ctx:b:b:true|") {
  throw "Set.prototype.forEach should call callback with value, value, set";
}
if (returned !== undefined) {
  throw "Set.prototype.forEach should return undefined";
}
