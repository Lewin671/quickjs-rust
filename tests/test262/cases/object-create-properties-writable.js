// Derived from: test/built-ins/Object/create/15.2.3.5-4-1.js
var object = Object.create({}, {
  fixed: { value: 1 },
  mutable: { value: 2, writable: true }
});
object.fixed = 3;
object.mutable = 4;
if (object.fixed !== 1) { throw; }
if (object.mutable !== 4) { throw; }
