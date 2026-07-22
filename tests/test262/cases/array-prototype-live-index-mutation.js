// Derived from: test/built-ins/Object/setPrototypeOf/success.js
var prototype = [];
var array = [];
var log = "";

Object.setPrototypeOf(array, prototype);
if (Object.getPrototypeOf(array) !== prototype) {
  throw "expected the original array prototype identity";
}

Object.defineProperty(prototype, "0", {
  set: function() { log += "first"; },
  configurable: true
});
array[0] = 1;

Object.defineProperty(prototype, "0", {
  set: function() { log += ":second"; },
  configurable: true
});
array[0] = 2;

delete prototype[0];
array[0] = 3;

if (log !== "first:second") {
  throw "expected added and replaced setters on the live array prototype";
}
if (array[0] !== 3 || !Object.prototype.hasOwnProperty.call(array, "0")) {
  throw "expected deletion of the inherited setter to restore an own index write";
}
