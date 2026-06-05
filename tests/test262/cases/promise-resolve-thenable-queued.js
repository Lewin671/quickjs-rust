// Derived from: test/built-ins/Promise/resolve/S25.4.4.5_A3.1_T1.js
var calls = 0;
var thenable = {
  then: function(resolve) {
    calls = calls + 1;
    resolve(1);
  }
};
var promise = Promise.resolve(thenable);
if (!(promise instanceof Promise)) {
  throw "Promise.resolve should return a Promise for thenables";
}
if (calls !== 0) {
  throw "thenable.then should not be called synchronously";
}
