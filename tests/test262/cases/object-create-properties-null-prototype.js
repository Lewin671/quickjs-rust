// Derived from: test/built-ins/Object/create/15.2.3.5-4-1.js
var object = Object.create(null, {
  own: { value: 2, enumerable: true }
});
if (Object.getPrototypeOf(object) !== null) { throw; }
if (object.own !== 2) { throw; }
if (Object.keys(object)[0] !== "own") { throw; }
