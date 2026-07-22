// Derived from: test/built-ins/ThrowTypeError/distinct-cross-realm.js
var localFunctionPrototype = Function.prototype;
var other = $262.createRealm().global;
var realmFunction = other.Function;
var originalFunctionPrototype = realmFunction.prototype;
var originalTypeErrorPrototype = other.TypeError.prototype;
var replacementFunctionPrototype = function ReplacementFunctionPrototype() {};

// `%Function.prototype%` is an intrinsic identity, independent of later writes
// to the Realm's mutable `Function` binding and its `prototype` property.
other.Function.prototype = replacementFunctionPrototype;
var functionStrict = other.Function('"use strict"; return arguments;');
var functionNonSimple = other.Function('value = 0', 'return arguments;');
other.Function = function ReplacementFunction() {};

var evalStrict = other.eval('(function() { "use strict"; return arguments; })');
var evalNonSimple = other.eval('(function(value = 0) { return arguments; })');
var evalGenerator = other.eval(
  '(function*() { "use strict"; return arguments; })'
);
var evalAsyncGenerator = other.eval(
  '(async function*() { "use strict"; return arguments; })'
);

[
  functionStrict,
  functionNonSimple,
  evalStrict,
  evalNonSimple,
  evalGenerator,
  evalAsyncGenerator
].forEach(function(fn) {
  if (fn.__quickjsRustRealmFunctionPrototype !== originalFunctionPrototype) {
    throw new Error('function lost its stable Realm Function prototype marker');
  }
});

function restrictedCallee(argumentsObject, label) {
  var descriptor = Object.getOwnPropertyDescriptor(argumentsObject, 'callee');
  if (descriptor.get !== descriptor.set) {
    throw new Error(label + ' did not use one poison function');
  }
  return descriptor.get;
}

var generatorResult = evalGenerator().next();
if (generatorResult.done !== true) {
  throw new Error('generator did not return its arguments object');
}

var throwers = [
  restrictedCallee(functionStrict(), 'Function strict'),
  restrictedCallee(functionNonSimple(), 'Function non-simple'),
  restrictedCallee(evalStrict(), 'eval strict'),
  restrictedCallee(evalNonSimple(), 'eval non-simple'),
  restrictedCallee(generatorResult.value, 'eval generator strict')
];

throwers.forEach(function(thrower) {
  if (thrower !== throwers[0]) {
    throw new Error('Realm created more than one %ThrowTypeError%');
  }
  if (Object.getPrototypeOf(thrower) !== originalFunctionPrototype) {
    throw new Error('%ThrowTypeError% used the wrong Function prototype');
  }
  if (
    Object.getPrototypeOf(thrower) === replacementFunctionPrototype ||
    Object.getPrototypeOf(thrower) === localFunctionPrototype
  ) {
    throw new Error('%ThrowTypeError% leaked a mutable or caller Realm prototype');
  }

  var thrown;
  try {
    thrower();
  } catch (error) {
    thrown = error;
  }
  if (!thrown || Object.getPrototypeOf(thrown) !== originalTypeErrorPrototype) {
    throw new Error('%ThrowTypeError% threw from the wrong Realm');
  }
});
