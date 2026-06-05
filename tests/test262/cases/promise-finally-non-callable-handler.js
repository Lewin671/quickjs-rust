// Derived from: test/built-ins/Promise/prototype/finally/invokes-then-with-non-function.js
var receiver = {
  then: function(onFulfilled, onRejected) {
    if (this !== receiver) {
      throw "finally should call receiver.then with the receiver as this";
    }
    if (onFulfilled !== 1) {
      throw "finally should pass non-callable handlers through";
    }
    if (onRejected !== 1) {
      throw "finally should pass non-callable handlers through";
    }
    return 42;
  }
};
if (Promise.prototype.finally.call(receiver, 1) !== 42) {
  throw "finally should return the receiver.then result";
}
