// Derived from: test/built-ins/Error/tostring-1.js
if (new Error().toString() !== "Error") { throw; }
if (new Error("boom").toString() !== "Error: boom") { throw; }
var value = new Error("boom");
value.name = "Custom";
if (value.toString() !== "Custom: boom") { throw; }
Error.prototype.toString = Object.prototype.toString;
if (new Error("boom").toString() !== "[object Error]") { throw; }
