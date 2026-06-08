// Derived from: test/built-ins/Array/prototype/fill/call-with-boolean.js
// Derived from: test/built-ins/Array/prototype/fill/return-this.js
// Derived from: test/built-ins/Array/prototype/fill/length-near-integer-limit.js
// Derived from: test/built-ins/Array/prototype/fill/return-abrupt-from-setting-property-value.js
if (!(Array.prototype.fill.call(true) instanceof Boolean)) {
  throw "Array.prototype.fill should return a boxed boolean receiver";
}

var object = { length: 4 };
var result = Array.prototype.fill.call(object, "x", 1, 3);
if (result !== object) {
  throw "Array.prototype.fill should return the generic receiver";
}
if (object.hasOwnProperty("0") || object[1] !== "x" || object[2] !== "x" || object.hasOwnProperty("3")) {
  throw "Array.prototype.fill should set only the requested generic indexes";
}

var value = {};
var start = Number.MAX_SAFE_INTEGER - 3;
var large = { length: Number.MAX_SAFE_INTEGER };
Array.prototype.fill.call(large, value, start, start + 3);
if (large[start] !== value || large[start + 1] !== value || large[start + 2] !== value) {
  throw "Array.prototype.fill should handle array-like indexes near the integer limit";
}

var throws = { length: 1 };
Object.defineProperty(throws, "0", {
  set: function() {
    throw new TypeError("setter failed");
  }
});
var caught = false;
try {
  Array.prototype.fill.call(throws);
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught) {
  throw "Array.prototype.fill should propagate property set failures";
}
