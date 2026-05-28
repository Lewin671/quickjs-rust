// Derived from: test/built-ins/Object/getOwnPropertyDescriptors/inherited-properties-omitted.js
var object = Object.create({ inherited: 1 }, { own: { value: 2, enumerable: true } });
var descriptors = Object.getOwnPropertyDescriptors(object);
if (Object.keys(descriptors).length !== 1 || descriptors.own.value !== 2 || descriptors.inherited !== undefined) {
  throw "Object.getOwnPropertyDescriptors should omit inherited properties";
}
