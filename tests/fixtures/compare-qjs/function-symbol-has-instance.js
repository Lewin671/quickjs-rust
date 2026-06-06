(function () {
  function C() {}
  var instance = new C();
  var descriptor = Object.getOwnPropertyDescriptor(Function.prototype, Symbol.hasInstance);
  var builtin = Function.prototype[Symbol.hasInstance];
  var objectCalls = 0;
  var objectMatcher = {};
  objectMatcher[Symbol.hasInstance] = function (value) {
    objectCalls = objectCalls + (this === objectMatcher ? 1 : 0);
    return value === 7;
  };
  function FunctionMatcher() {}
  Object.defineProperty(FunctionMatcher, Symbol.hasInstance, {
    value: function (value) {
      return value === 3 ? "yes" : "";
    },
    configurable: true,
  });
  var caught = false;
  var notCallable = {};
  notCallable[Symbol.hasInstance] = 1;
  try {
    1 instanceof notCallable;
  } catch (error) {
    caught = error instanceof TypeError;
  }
  return [
    typeof descriptor.value,
    descriptor.writable,
    descriptor.enumerable,
    descriptor.configurable,
    builtin.length,
    builtin.name,
    builtin.call(C, instance),
    builtin.call(C, {}),
    builtin.call({}, {}),
    7 instanceof objectMatcher,
    8 instanceof objectMatcher,
    objectCalls,
    3 instanceof FunctionMatcher,
    4 instanceof FunctionMatcher,
    FunctionMatcher[Symbol.hasInstance] === Function.prototype[Symbol.hasInstance],
    caught,
  ].join(":");
})()
