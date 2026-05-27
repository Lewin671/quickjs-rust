// Derived from: test/built-ins/Object/create/15.2.3.5-4-1.js
var proto = { inherited: 1 };
var object = Object.create(proto, {
  own: { value: 2, enumerable: true },
  hidden: { value: 3 }
});
if (Object.getPrototypeOf(object) !== proto) { throw; }
if (object.inherited !== 1) { throw; }
if (object.own !== 2) { throw; }
if (object.hidden !== 3) { throw; }
if (Object.keys(object).length !== 1) { throw; }
