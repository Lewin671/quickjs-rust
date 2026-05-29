// Derived from: test/built-ins/NativeErrors/TypeError/prototype.js
var nativeErrors = [EvalError, RangeError, ReferenceError, SyntaxError, TypeError, URIError];
var names = ["EvalError", "RangeError", "ReferenceError", "SyntaxError", "TypeError", "URIError"];
for (var i = 0; i < nativeErrors.length; i++) {
  var value = new nativeErrors[i]("boom");
  if (value.toString() !== names[i] + ": boom") { throw; }
  if (Object.prototype.toString.call(value) !== "[object Error]") { throw; }
}
