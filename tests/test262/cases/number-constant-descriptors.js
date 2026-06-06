// Derived from: test/built-ins/Number/EPSILON.js
// Derived from: test/built-ins/Number/MAX_SAFE_INTEGER.js
// Derived from: test/built-ins/Number/MIN_SAFE_INTEGER.js
// Derived from: test/built-ins/Number/NEGATIVE_INFINITY/prop-desc.js
// Derived from: test/built-ins/Number/POSITIVE_INFINITY/prop-desc.js
var constants = [
  ["EPSILON", Number.EPSILON],
  ["MAX_SAFE_INTEGER", 9007199254740991],
  ["MIN_SAFE_INTEGER", -9007199254740991],
  ["NEGATIVE_INFINITY", -Infinity],
  ["POSITIVE_INFINITY", Infinity]
];

for (var i = 0; i < constants.length; i++) {
  var name = constants[i][0];
  var value = constants[i][1];
  var descriptor = Object.getOwnPropertyDescriptor(Number, name);
  if (descriptor.value !== value) {
    throw "unexpected Number constant value";
  }
  if (descriptor.writable || descriptor.enumerable || descriptor.configurable) {
    throw "Number constant descriptor should be frozen data";
  }
  Number[name] = 1;
  if (Number[name] !== value) {
    throw "Number constant should not be writable";
  }
  if (delete Number[name]) {
    throw "Number constant should not be configurable";
  }
}
