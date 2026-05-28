// Derived from: test/built-ins/Object/setPrototypeOf/success.js
var object = {};
Object.setPrototypeOf(object, null);
if (Object.getPrototypeOf(object) !== null) throw new Error("prototype should be null");
