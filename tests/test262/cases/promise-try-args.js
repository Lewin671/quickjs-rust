// Derived from: test/built-ins/Promise/try/args.js
var promise = Promise.try(function(a, b, c) {
  return String(a) + ":" + b + ":" + (c === undefined);
}, 1, 2);
promise.then(function(value) {
  if (value !== "1:2:true") {
    throw "Promise.try should forward arguments after callback";
  }
});
