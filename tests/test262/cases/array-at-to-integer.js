// Derived from: test/built-ins/Array/prototype/at/index-argument-tointeger.js
var array = [1, 2, 3];

if (array.at(1.9) !== 2) {
  throw "expected at(1.9) to truncate toward zero";
}
if (array.at(-1.9) !== 3) {
  throw "expected at(-1.9) to truncate toward zero before resolving relative index";
}
if (array.at() !== 1) {
  throw "expected omitted index to behave like 0";
}

var valueOfCallCount = 0;
var index = {
  valueOf: function() {
    valueOfCallCount += 1;
    return 1;
  }
};

if (array.at(index) !== 2) {
  throw "expected at({ valueOf }) to use the coerced integer";
}
if (valueOfCallCount !== 1) {
  throw "expected at({ valueOf }) to call valueOf exactly once";
}
