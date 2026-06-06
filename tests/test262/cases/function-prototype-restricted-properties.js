// Derived from: test/built-ins/Function/prototype/arguments/prop-desc.js
var argumentsDesc = Object.getOwnPropertyDescriptor(Function.prototype, "arguments");
var callerDesc = Object.getOwnPropertyDescriptor(Function.prototype, "caller");

if (argumentsDesc.enumerable !== false || argumentsDesc.configurable !== true) {
  throw new Error("expected Function.prototype.arguments descriptor flags");
}
if (typeof argumentsDesc.get !== "function" || argumentsDesc.get !== argumentsDesc.set) {
  throw new Error("expected Function.prototype.arguments getter and setter to be one function");
}
if (argumentsDesc.get !== callerDesc.get || callerDesc.get !== callerDesc.set) {
  throw new Error("expected Function.prototype caller and arguments to share %ThrowTypeError%");
}

try {
  Function.prototype.arguments;
  throw new Error("expected Function.prototype.arguments getter to throw");
} catch (error) {
  if (!(error instanceof TypeError)) {
    throw error;
  }
}

try {
  Function.prototype.arguments = 1;
  throw new Error("expected Function.prototype.arguments setter to throw");
} catch (error) {
  if (!(error instanceof TypeError)) {
    throw error;
  }
}
