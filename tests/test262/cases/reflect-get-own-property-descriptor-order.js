// Derived from: test/built-ins/Reflect/getOwnPropertyDescriptor/return-from-data-descriptor.js
// Derived from: test/built-ins/Reflect/getOwnPropertyDescriptor/return-from-accessor-descriptor.js
// Derived from: test/built-ins/Reflect/getOwnPropertyDescriptor/symbol-property.js
function assertSameArray(actual, expected) {
  if (actual.length !== expected.length) {
    throw "expected array length " + expected.length;
  }
  for (var i = 0; i < expected.length; i++) {
    if (actual[i] !== expected[i]) {
      throw "expected " + expected[i] + " at " + i;
    }
  }
}

var dataObject = {
  p: "foo"
};
var dataDescriptor = Reflect.getOwnPropertyDescriptor(dataObject, "p");
assertSameArray(
  Object.getOwnPropertyNames(dataDescriptor),
  ["value", "writable", "enumerable", "configurable"]
);
if (dataDescriptor.value !== "foo") {
  throw "expected data descriptor value";
}
if (!dataDescriptor.writable || !dataDescriptor.enumerable || !dataDescriptor.configurable) {
  throw "expected data descriptor flags";
}

var accessorObject = {};
var getter = function() {};
Object.defineProperty(accessorObject, "p", {
  get: getter,
  configurable: true
});
var accessorDescriptor = Reflect.getOwnPropertyDescriptor(accessorObject, "p");
assertSameArray(
  Object.getOwnPropertyNames(accessorDescriptor),
  ["get", "set", "enumerable", "configurable"]
);
if (accessorDescriptor.get !== getter || accessorDescriptor.set !== undefined) {
  throw "expected accessor descriptor functions";
}
if (accessorDescriptor.enumerable || !accessorDescriptor.configurable) {
  throw "expected accessor descriptor flags";
}

var symbolObject = {};
var symbol = Symbol("p");
symbolObject[symbol] = 42;
var symbolDescriptor = Reflect.getOwnPropertyDescriptor(symbolObject, symbol);
assertSameArray(
  Object.getOwnPropertyNames(symbolDescriptor),
  ["value", "writable", "enumerable", "configurable"]
);
if (symbolDescriptor.value !== 42) {
  throw "expected symbol descriptor value";
}
