// Derived from: test/built-ins/Object/getOwnPropertyDescriptors/primitive-strings.js
var descriptors = Object.getOwnPropertyDescriptors("abc");
if (Object.keys(descriptors).length !== 4) {
  throw "Object.getOwnPropertyDescriptors should expose string index and length descriptors";
}
if (descriptors.length.value !== 3 || descriptors.length.enumerable !== false || descriptors.length.writable !== false || descriptors.length.configurable !== false) {
  throw "Object.getOwnPropertyDescriptors should expose a string length descriptor";
}
if (descriptors[0].value !== "a" || descriptors[0].enumerable !== true || descriptors[0].writable !== false || descriptors[0].configurable !== false) {
  throw "Object.getOwnPropertyDescriptors should expose string index descriptors";
}
