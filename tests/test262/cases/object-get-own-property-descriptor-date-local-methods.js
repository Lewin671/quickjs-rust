// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-121.js
// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-138.js
// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-152.js
function checkDatePrototypeMethod(name) {
  var desc = Object.getOwnPropertyDescriptor(Date.prototype, name);
  if (desc.value !== Date.prototype[name]) { throw name + " value"; }
  if (desc.writable !== true) { throw name + " writable"; }
  if (desc.enumerable !== false) { throw name + " enumerable"; }
  if (desc.configurable !== true) { throw name + " configurable"; }
}

checkDatePrototypeMethod("getMonth");
checkDatePrototypeMethod("getDate");
checkDatePrototypeMethod("getDay");
checkDatePrototypeMethod("getHours");
checkDatePrototypeMethod("getMinutes");
checkDatePrototypeMethod("getSeconds");
checkDatePrototypeMethod("getMilliseconds");
checkDatePrototypeMethod("setFullYear");
checkDatePrototypeMethod("setMonth");
checkDatePrototypeMethod("setDate");
checkDatePrototypeMethod("setHours");
checkDatePrototypeMethod("setMinutes");
checkDatePrototypeMethod("setSeconds");
checkDatePrototypeMethod("setMilliseconds");
checkDatePrototypeMethod("toLocaleString");
checkDatePrototypeMethod("toLocaleDateString");
checkDatePrototypeMethod("toLocaleTimeString");
