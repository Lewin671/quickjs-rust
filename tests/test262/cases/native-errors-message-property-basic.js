// Derived from: test/built-ins/NativeErrors/message_property_native_error.js
var nativeErrors = [EvalError, RangeError, ReferenceError, SyntaxError, TypeError, URIError];
for (var i = 0; i < nativeErrors.length; i++) {
  var value = new nativeErrors[i]("my-message");
  var descriptor = Object.getOwnPropertyDescriptor(value, "message");
  if (value.message !== "my-message") { throw; }
  if (descriptor.value !== "my-message") { throw; }
  if (descriptor.writable !== true) { throw; }
  if (descriptor.enumerable !== false) { throw; }
  if (descriptor.configurable !== true) { throw; }
}
