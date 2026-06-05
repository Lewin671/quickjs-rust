// Derived from: test/built-ins/Promise/resolve-thenable-immed.js
var calls = 0;
var returnValue = "unset";
var thenable = {
  then: function(resolve) {
    calls = calls + 1;
    resolve(1);
  }
};
new Promise(function(resolve) {
  returnValue = resolve(thenable);
});
if (returnValue !== undefined) {
  throw "Promise resolving functions should return undefined";
}
if (calls !== 0) {
  throw "thenable.then should not be called synchronously";
}
