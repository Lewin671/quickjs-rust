// Derived from: test/built-ins/Promise/resolve/S25.4.4.5_A3.1_T1.js
var getterCalls = 0;
var thenCalls = 0;
var thenable = {};
Object.defineProperty(thenable, "then", {
  get: function() {
    getterCalls = getterCalls + 1;
    return function(resolve) {
      thenCalls = thenCalls + 1;
      resolve(1);
    };
  }
});
var promise = Promise.resolve(thenable);
if (!(promise instanceof Promise)) {
  throw "Promise.resolve should return a Promise for thenables";
}
if (getterCalls !== 1) {
  throw "Promise.resolve should read then synchronously";
}
if (thenCalls !== 0) {
  throw "thenable.then should not be called synchronously";
}
