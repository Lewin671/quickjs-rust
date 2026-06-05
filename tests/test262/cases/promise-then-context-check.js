// Derived from: test/built-ins/Promise/prototype/then/context-check-on-entry.js
var threwObject = false;
try {
  Promise.prototype.then.call({});
} catch (error) {
  threwObject = error instanceof TypeError;
}
if (!threwObject) {
  throw "Promise.prototype.then should reject non-Promise object receivers";
}

var threwPrimitive = false;
try {
  Promise.prototype.then.call(3);
} catch (error) {
  threwPrimitive = error instanceof TypeError;
}
if (!threwPrimitive) {
  throw "Promise.prototype.then should reject primitive receivers";
}
