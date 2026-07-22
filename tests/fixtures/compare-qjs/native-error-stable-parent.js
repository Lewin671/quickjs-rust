(function () {
  var original = Error;
  Error = function FakeError() {};
  return [
    EvalError,
    RangeError,
    ReferenceError,
    SyntaxError,
    TypeError,
    URIError,
    AggregateError,
    SuppressedError
  ].map(function (constructor) {
    return Object.getPrototypeOf(constructor) === original;
  }).join(":");
})()
