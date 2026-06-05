// Derived from: test/built-ins/Promise/allSettled/resolve-thenable.js
var thenCalls = 0;
var thenable = {
  then: function(resolve) {
    thenCalls += 1;
    resolve(1);
  }
};
var promise = Promise.allSettled([thenable]);
if (!(promise instanceof Promise)) {
  throw "Promise.allSettled should return a Promise for thenables";
}
if (thenCalls !== 0) {
  throw "Promise.allSettled should not call thenable bodies synchronously";
}
