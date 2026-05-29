// Derived from: test/built-ins/NativeErrors/TypeError/name.js
var names = ["EvalError", "RangeError", "ReferenceError", "SyntaxError", "TypeError", "URIError"];
for (var i = 0; i < names.length; i++) {
  var name = names[i];
  var NativeError = this[name];
  if (NativeError.name !== name) { throw; }
  if (NativeError.prototype.name !== name) { throw; }
}
