// Derived from: test/built-ins/Object/create/15.2.3.5-4-2.js
var object = Object.create({}, undefined);
if (!(object instanceof Object)) { throw; }
if (Object.keys(object).length !== 0) { throw; }
