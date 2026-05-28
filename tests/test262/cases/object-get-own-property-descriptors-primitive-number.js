// Derived from: test/built-ins/Object/getOwnPropertyDescriptors/primitive-numbers.js
if (Object.keys(Object.getOwnPropertyDescriptors(0)).length !== 0) {
  throw "Object.getOwnPropertyDescriptors should return no descriptors for numbers";
}
if (Object.keys(Object.getOwnPropertyDescriptors(NaN)).length !== 0) {
  throw "Object.getOwnPropertyDescriptors should return no descriptors for NaN";
}
