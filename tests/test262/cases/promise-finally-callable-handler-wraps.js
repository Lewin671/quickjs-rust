// Derived from: test/built-ins/Promise/prototype/finally/invokes-then-with-function.js
var onFinally = function() {};
var receiver = {
  then: function(onFulfilled, onRejected) {
    if (this !== receiver) {
      throw "finally should call receiver.then with the receiver as this";
    }
    if (typeof onFulfilled !== "function") {
      throw "finally should pass a fulfillment wrapper";
    }
    if (typeof onRejected !== "function") {
      throw "finally should pass a rejection wrapper";
    }
    if (onFulfilled === onFinally || onRejected === onFinally) {
      throw "finally should wrap callable handlers";
    }
    if (onFulfilled.length !== 1 || onRejected.length !== 1) {
      throw "finally wrapper length should be 1";
    }
    return 42;
  }
};
if (Promise.prototype.finally.call(receiver, onFinally) !== 42) {
  throw "finally should return the receiver.then result";
}
