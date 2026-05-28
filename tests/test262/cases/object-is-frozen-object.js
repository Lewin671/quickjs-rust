// Derived from: test/built-ins/Object/isFrozen/15.2.3.12-2-1.js
var object = {};
if (Object.isFrozen(object) !== false) throw new Error("ordinary object should start unfrozen");
Object.freeze(object);
if (Object.isFrozen(object) !== true) throw new Error("object should be frozen");
