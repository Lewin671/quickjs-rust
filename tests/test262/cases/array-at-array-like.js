// Derived from: test/built-ins/Array/prototype/at/return-abrupt-from-this.js
// Derived from: test/built-ins/Array/prototype/at/returns-item.js
if (Array.prototype.at.call("abc", -2) !== "b") {
  throw "expected Array.prototype.at to read from string receivers";
}

var object = { length: 2, 1: "x" };
if (Array.prototype.at.call(object, 1) !== "x") {
  throw "expected Array.prototype.at to read from array-like objects";
}

var lengthAccessed = false;
var abrupt = {};
var throwing = {};
Object.defineProperty(throwing, "length", {
  get: function() {
    lengthAccessed = true;
    throw abrupt;
  }
});

try {
  Array.prototype.at.call(throwing, 0);
  throw "expected length getter abrupt completion";
} catch (error) {
  if (error !== abrupt) {
    throw "expected length getter error to be propagated";
  }
}

if (!lengthAccessed) {
  throw "expected length getter to be called";
}
