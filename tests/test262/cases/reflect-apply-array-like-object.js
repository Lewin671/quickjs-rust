// Derived from: test/built-ins/Reflect/apply/arguments-list-is-not-array-like-but-still-valid.js
var object = {};
Object.defineProperty(object, "length", {
  get: function() {
    return 1;
  }
});

var result = Reflect.apply(function(value) {
  return [value, arguments.length];
}, null, object);

if (result.length !== 2) {
  throw "expected result array";
}
if (result[0] !== undefined) {
  throw "expected missing array-like value to be undefined";
}
if (result[1] !== 1) {
  throw "expected one array-like argument";
}
