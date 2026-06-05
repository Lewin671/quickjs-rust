// Derived from: test/built-ins/Promise/any/invoke-then.js
var thenCalls = 0;
var thenable = {
  then: function(resolve) {
    thenCalls += 1;
    resolve(1);
  }
};
var promise = Promise.any([thenable]);
if (!(promise instanceof Promise)) {
  throw "Promise.any should return a Promise for thenables";
}
if (thenCalls !== 0) {
  throw "Promise.any should not call thenable bodies synchronously";
}
