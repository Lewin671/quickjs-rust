// Derived from: test/built-ins/AggregateError/prototype/name.js
if (AggregateError.prototype.name !== "AggregateError") {
  throw "AggregateError prototype should expose its name";
}
if (AggregateError.prototype.constructor !== AggregateError) {
  throw "AggregateError prototype constructor should point at AggregateError";
}
