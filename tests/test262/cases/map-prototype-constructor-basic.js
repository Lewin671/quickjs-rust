// Derived from: test/built-ins/Map/prototype/constructor.js
if (Map.prototype.constructor !== Map) {
  throw "Map.prototype.constructor should reference Map";
}
if (Object.prototype.toString.call(new Map()) !== "[object Map]") {
  throw "Map instances should have the Map toString tag";
}

