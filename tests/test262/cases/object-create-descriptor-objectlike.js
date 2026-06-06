// Derived from: test/built-ins/Object/create/15.2.3.5-0-2.js
// Derived from: test/built-ins/Object/create/15.2.3.5-4-105.js
// Derived from: test/built-ins/Object/create/15.2.3.5-4-112.js

if (Object.create.length !== 2) {
  throw "expected Object.create.length to be 2";
}

var accessorDescriptor = {};
Object.defineProperty(accessorDescriptor, "configurable", {
  get: function() {
    return true;
  }
});

var configurableObject = Object.create({}, {
  prop: accessorDescriptor
});

if (!configurableObject.hasOwnProperty("prop")) {
  throw "expected configurable property to be defined";
}
delete configurableObject.prop;
if (configurableObject.hasOwnProperty("prop")) {
  throw "expected configurable property to delete";
}

var functionDescriptor = function() {};
functionDescriptor.enumerable = true;
functionDescriptor.value = 7;

var functionDescriptorObject = Object.create({}, {
  prop: functionDescriptor
});

if (functionDescriptorObject.prop !== 7) {
  throw "expected function descriptor value";
}
if (Object.keys(functionDescriptorObject)[0] !== "prop") {
  throw "expected function descriptor enumerable property";
}
