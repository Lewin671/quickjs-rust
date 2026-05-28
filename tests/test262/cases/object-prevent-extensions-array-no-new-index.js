// Derived from: test/built-ins/Object/preventExtensions/15.2.3.10-3-2.js
let array = [1];
Object.preventExtensions(array);
array[1] = 2;
if (array.length !== 1) throw new Error("new array index should not be added");
