// Derived from: test/built-ins/String/prototype/matchAll/regexp-matchAll-invocation.js
if (typeof String.prototype.matchAll !== "function") {
  throw "expected String.prototype.matchAll to be a function";
}

var descriptor = Object.getOwnPropertyDescriptor(String.prototype, "matchAll");
if (!descriptor.writable || descriptor.enumerable || !descriptor.configurable) {
  throw "expected String.prototype.matchAll property descriptor attributes";
}

if (String.prototype.matchAll.length !== 1) {
  throw "expected String.prototype.matchAll length to be 1";
}

var object = {};
var returnValue = {};
var callCount = 0;
var thisValue;
var argument;

object[Symbol.matchAll] = function(input) {
  callCount = callCount + 1;
  thisValue = this;
  argument = input;
  return returnValue;
};

if ("".matchAll(object) !== returnValue) {
  throw "expected String.prototype.matchAll to return custom matcher result";
}
if (callCount !== 1) {
  throw "expected String.prototype.matchAll to call custom matcher once";
}
if (thisValue !== object) {
  throw "expected String.prototype.matchAll to call matcher with regexp receiver";
}
if (argument !== "") {
  throw "expected String.prototype.matchAll to pass string receiver";
}

var threw = false;
try {
  "".matchAll({ [Symbol.matchAll]: true });
} catch (error) {
  threw = error instanceof TypeError;
}
if (!threw) {
  throw "expected String.prototype.matchAll to reject non-callable Symbol.matchAll";
}
