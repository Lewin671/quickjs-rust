// Derived from: test/built-ins/Object/defineProperties/15.2.3.7-5-a-1.js
var object = {};
var result = Object.defineProperties(object, {
  first: { value: 1, enumerable: true, writable: true },
  second: { value: 2 }
});
if (result !== object) { throw; }
if (object.first !== 1) { throw; }
if (object.second !== 2) { throw; }
if (Object.keys(object).length !== 1) { throw; }
