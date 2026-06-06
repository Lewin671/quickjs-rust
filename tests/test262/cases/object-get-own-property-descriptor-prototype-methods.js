// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-35.js
// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-51.js
// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-69.js
// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-79.js
// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-80.js
// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-90.js
function checkPrototypeMethod(object, name) {
  var desc = Object.getOwnPropertyDescriptor(object, name);
  if (desc.value !== object[name]) { throw name + " value"; }
  if (desc.writable !== true) { throw name + " writable"; }
  if (desc.enumerable !== false) { throw name + " enumerable"; }
  if (desc.configurable !== true) { throw name + " configurable"; }
}

checkPrototypeMethod(Function.prototype, "toString");
checkPrototypeMethod(Array.prototype, "toLocaleString");
checkPrototypeMethod(String.prototype, "replace");
checkPrototypeMethod(String.prototype, "toLocaleLowerCase");
checkPrototypeMethod(String.prototype, "toLocaleUpperCase");
checkPrototypeMethod(Number.prototype, "toLocaleString");
