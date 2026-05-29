// Derived from: test/built-ins/NativeErrors/TypeError/constructor.js
var nativeErrors = [EvalError, RangeError, ReferenceError, SyntaxError, TypeError, URIError];
for (var i = 0; i < nativeErrors.length; i++) {
  var NativeError = nativeErrors[i];
  if (typeof NativeError !== "function") { throw; }
  if (NativeError.length !== 1) { throw; }
  var value = new NativeError("boom");
  if (value instanceof NativeError !== true) { throw; }
  if (value instanceof Error !== true) { throw; }
  if (value.constructor !== NativeError) { throw; }
}
