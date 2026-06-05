// Derived from: test/built-ins/Map/prototype/forEach/callback-parameters.js
var map = new Map();
map.set("a", 1);
map.set("b", 2);
var seen = "";
var thisArg = { marker: "ctx" };
var returned = map.forEach(function(value, key, receiver) {
  seen += this.marker + ":" + key + ":" + value + ":" + (receiver === map) + "|";
}, thisArg);
if (seen !== "ctx:a:1:true|ctx:b:2:true|") {
  throw "Map.prototype.forEach should call callback with value, key, map";
}
if (returned !== undefined) {
  throw "Map.prototype.forEach should return undefined";
}
