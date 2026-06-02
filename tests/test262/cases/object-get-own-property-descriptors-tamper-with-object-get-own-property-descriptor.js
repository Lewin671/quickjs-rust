// Derived from: test/built-ins/Object/getOwnPropertyDescriptors/tamper-with-object-keys.js
function fakeObjectGetOwnPropertyDescriptor() {
  throw "called";
}
Object.getOwnPropertyDescriptor = fakeObjectGetOwnPropertyDescriptor;

if (Object.getOwnPropertyDescriptor !== fakeObjectGetOwnPropertyDescriptor) {
  throw "Object.getOwnPropertyDescriptor was not replaced";
}
if (Object.keys(Object.getOwnPropertyDescriptors({ a: 1 })).length !== 1) {
  throw "expected object with one key to have one descriptor";
}
