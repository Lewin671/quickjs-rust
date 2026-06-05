// Derived from: test/built-ins/Error/isError/errors.js

if (Error.isError(new Error()) !== true) {
  throw "Error.isError should accept Error instances";
}
if (Error.isError(new TypeError()) !== true) {
  throw "Error.isError should accept native error instances";
}
if (Error.isError(new AggregateError([])) !== true) {
  throw "Error.isError should accept AggregateError instances";
}
if (Error.isError({}) !== false) {
  throw "Error.isError should reject ordinary objects";
}
if (Error.isError(Error) !== false) {
  throw "Error.isError should reject constructors";
}
if (Error.isError() !== false) {
  throw "Error.isError should reject missing arguments";
}
