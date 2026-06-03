// Derived from: test/built-ins/RegExp/prototype/global/15.10.7.2-2.js
var globalDesc = Object.getOwnPropertyDescriptor(RegExp.prototype, "global");
if (typeof globalDesc.get !== "function") { throw "global getter"; }
if (globalDesc.set !== undefined) { throw "global setter"; }
if (globalDesc.enumerable !== false) { throw "global enumerable"; }
if (globalDesc.configurable !== true) { throw "global configurable"; }

if (/a/g.global !== true) { throw "global true"; }
if (/a/.global !== false) { throw "global false"; }
if (/a/i.ignoreCase !== true) { throw "ignoreCase true"; }
if (/a/m.multiline !== true) { throw "multiline true"; }

var get = globalDesc.get;
if (get.call(RegExp.prototype) !== undefined) {
  throw "RegExp.prototype.global getter should special-case RegExp.prototype";
}

var caught = false;
try {
  get.call({});
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught) {
  throw "RegExp.prototype.global getter should reject ordinary objects";
}
