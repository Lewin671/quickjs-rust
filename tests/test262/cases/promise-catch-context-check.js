// Derived from: test/built-ins/Promise/prototype/catch/invokes-then.js
var threwObject = false;
try {
  Promise.prototype.catch.call({});
} catch (error) {
  threwObject = error instanceof TypeError;
}
if (!threwObject) {
  throw "Promise.prototype.catch should reject non-Promise object receivers";
}

var threwPrimitive = false;
try {
  Promise.prototype.catch.call(3);
} catch (error) {
  threwPrimitive = error instanceof TypeError;
}
if (!threwPrimitive) {
  throw "Promise.prototype.catch should reject primitive receivers";
}
