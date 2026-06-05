// Derived from: test/built-ins/Promise/race/resolve-thenable.js
var calls = 0;
var thenable = {
  then: function(resolve) {
    calls = calls + 1;
    resolve(1);
  }
};
var promise = Promise.race([thenable]);
if (!(promise instanceof Promise)) {
  throw "Promise.race should return a Promise for thenables";
}
if (calls !== 0) {
  throw "thenable.then should not be called synchronously";
}
