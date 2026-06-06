// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-4.js
// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-10.js
// Derived from: test/built-ins/Object/getOwnPropertyDescriptor/15.2.3.3-4-180.js
function checkWritableGlobal(name) {
  var desc = Object.getOwnPropertyDescriptor(this, name);
  if (desc.value !== this[name]) { throw name + " value"; }
  if (desc.writable !== true) { throw name + " writable"; }
  if (desc.enumerable !== false) { throw name + " enumerable"; }
  if (desc.configurable !== true) { throw name + " configurable"; }
}

checkWritableGlobal("eval");
checkWritableGlobal("decodeURIComponent");
checkWritableGlobal("Object");
checkWritableGlobal("parseInt");

var undefinedDesc = Object.getOwnPropertyDescriptor(this, "undefined");
if (undefinedDesc.writable !== false) { throw "undefined writable"; }
if (undefinedDesc.enumerable !== false) { throw "undefined enumerable"; }
if (undefinedDesc.configurable !== false) { throw "undefined configurable"; }
if (undefinedDesc.hasOwnProperty("get")) { throw "undefined get"; }
if (undefinedDesc.hasOwnProperty("set")) { throw "undefined set"; }
