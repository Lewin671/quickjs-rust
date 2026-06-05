// Derived from: test/built-ins/AggregateError/is-a-constructor.js
if (typeof AggregateError !== "function") {
  throw "AggregateError should be a function";
}
if (AggregateError.length !== 2) {
  throw "AggregateError.length should be 2";
}
if (!(new AggregateError([]) instanceof AggregateError)) {
  throw "AggregateError should construct AggregateError instances";
}
