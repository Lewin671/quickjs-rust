// Derived from: test/built-ins/AggregateError/errors-iterabletolist.js
var error = new AggregateError([1, 2], "boom");
if (error.errors.length !== 2) {
  throw "AggregateError errors should keep input length";
}
if (error.errors[0] !== 1 || error.errors[1] !== 2) {
  throw "AggregateError errors should keep input values";
}
if (error.message !== "boom") {
  throw "AggregateError should keep message";
}
