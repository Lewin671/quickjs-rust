// Derived from: test/built-ins/Map/constructor.js
if (typeof Map !== "function") {
  throw "Map should be a function";
}
if (Map.length !== 0) {
  throw "Map.length should be 0";
}
if (!(new Map() instanceof Map)) {
  throw "new Map should create Map instances";
}

