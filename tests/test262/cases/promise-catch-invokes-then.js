// Derived from: test/built-ins/Promise/prototype/catch/invokes-then.js
var receiver = {
  then: function(onFulfilled, onRejected) {
    if (this !== receiver) {
      throw "catch should call receiver.then with the receiver as this";
    }
    if (onFulfilled !== undefined) {
      throw "catch should pass undefined as the fulfillment handler";
    }
    if (typeof onRejected !== "function") {
      throw "catch should pass the rejection handler";
    }
    return 42;
  }
};
if (Promise.prototype.catch.call(receiver, function() {}) !== 42) {
  throw "catch should return the receiver.then result";
}
