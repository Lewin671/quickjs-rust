// Derived from: test/built-ins/String/prototype/match/this-value-not-obj-coercible.js
var match = String.prototype.match;
var undefinedCaught = false;
try {
  match.call(undefined, /./);
} catch (error) {
  undefinedCaught = error instanceof TypeError;
}
if (!undefinedCaught) {
  throw "expected undefined this value to throw TypeError";
}

var nullCaught = false;
try {
  match.call(null, /./);
} catch (error) {
  nullCaught = error instanceof TypeError;
}
if (!nullCaught) {
  throw "expected null this value to throw TypeError";
}
