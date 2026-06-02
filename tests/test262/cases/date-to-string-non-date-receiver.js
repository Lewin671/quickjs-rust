// Derived from: test/built-ins/Date/prototype/toString/non-date-receiver.js
var caught = false;
try {
  Date.prototype.toString();
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught) {
  throw "expected Date.prototype.toString to reject Date.prototype receiver";
}
