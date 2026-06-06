// Derived from: test/built-ins/Object/defineProperties/15.2.3.7-5-a-8.js
var object = {};
var descriptors = [];
descriptors.prop = { value: 8, enumerable: true };
Object.defineProperties(object, descriptors);

if (!object.hasOwnProperty("prop")) { throw; }
if (object.prop !== 8) { throw; }

var getterCalled = false;
Object.defineProperty(descriptors, "computed", {
  get: function() {
    getterCalled = true;
    return { value: 9, enumerable: true };
  },
  enumerable: true
});
Object.defineProperties(object, descriptors);

if (!getterCalled) { throw; }
if (object.computed !== 9) { throw; }
