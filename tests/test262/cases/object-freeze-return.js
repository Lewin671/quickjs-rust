// Derived from: test/built-ins/Object/freeze/15.2.3.9-3-1.js
var object = {};
if (Object.freeze(object) !== object) throw new Error("should return target");
if (Object.isFrozen(object) !== true) throw new Error("frozen object should be frozen");
