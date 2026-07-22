// Derived from: test/staging/sm/Function/arguments-iterator.js
var localArrayValues = Array.prototype.values;
var otherGlobal = $262.createRealm().global;
var otherArrayValues = otherGlobal.Array.prototype.values;

if (typeof otherArrayValues !== "function") { throw new Error("missing realm intrinsic"); }
if (otherArrayValues === localArrayValues) { throw new Error("shared realm intrinsic"); }

var mapped = otherGlobal.Function("return arguments[Symbol.iterator];");
var unmapped = otherGlobal.Function('"use strict"; return arguments[Symbol.iterator];');

if (mapped(1) !== otherArrayValues) { throw new Error("mapped iterator realm"); }
if (unmapped(1) !== otherArrayValues) { throw new Error("unmapped iterator realm"); }
if (mapped(1) === localArrayValues) { throw new Error("mapped iterator leaked caller realm"); }
if (unmapped(1) === localArrayValues) { throw new Error("unmapped iterator leaked caller realm"); }

var iterator = mapped(1).call({ 0: "first", length: 1 });
var step = iterator.next();
if (step.value !== "first" || step.done !== false) { throw new Error("iterator is not callable"); }

otherGlobal.Array.prototype.values = function replacement() {};
otherGlobal.Array = function ReplacementArray() {};
if (mapped(1) !== otherArrayValues) { throw new Error("mutable global replaced intrinsic"); }
if (unmapped(1) !== otherArrayValues) { throw new Error("mutable global replaced strict intrinsic"); }
