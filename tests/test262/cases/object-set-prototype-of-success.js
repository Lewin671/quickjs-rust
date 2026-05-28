// Derived from: test/built-ins/Object/setPrototypeOf/success.js
var propValue = {};
var newProto = { test262prop: propValue };
var object = {};
if (Object.setPrototypeOf(object, newProto) !== object) throw new Error("should return target");
if (object.test262prop !== propValue) throw new Error("prototype property should be inherited");
