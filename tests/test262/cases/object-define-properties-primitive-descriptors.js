// Derived from: test/built-ins/Object/defineProperties/15.2.3.7-2-3.js
// Derived from: test/built-ins/Object/defineProperties/15.2.3.7-2-5.js
// Derived from: test/built-ins/Object/defineProperties/15.2.3.7-2-7.js

var booleanTarget = {};
if (Object.defineProperties(booleanTarget, false) !== booleanTarget) {
  throw "expected boolean properties argument to return target";
}
if (Object.keys(booleanTarget).length !== 0) {
  throw "expected boolean properties argument to define no properties";
}

var numberTarget = { value: 1 };
if (Object.defineProperties(numberTarget, -12) !== numberTarget) {
  throw "expected number properties argument to return target";
}
if (numberTarget.value !== 1) {
  throw "expected number properties argument to preserve properties";
}

var stringTarget = { value: 1 };
if (Object.defineProperties(stringTarget, "") !== stringTarget) {
  throw "expected empty string properties argument to return target";
}
if (stringTarget.value !== 1) {
  throw "expected empty string properties argument to preserve properties";
}
