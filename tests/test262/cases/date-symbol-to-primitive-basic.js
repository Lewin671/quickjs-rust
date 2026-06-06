// Derived from: test/built-ins/Date/prototype/Symbol.toPrimitive/prop-desc.js
// Derived from: test/built-ins/Date/prototype/Symbol.toPrimitive/length.js
// Derived from: test/built-ins/Date/prototype/Symbol.toPrimitive/name.js
// Derived from: test/built-ins/Date/prototype/Symbol.toPrimitive/hint-default-first-invalid.js
// Derived from: test/built-ins/Date/prototype/Symbol.toPrimitive/hint-number-first-valid.js
// Derived from: test/built-ins/Date/prototype/Symbol.toPrimitive/hint-invalid.js
// Derived from: test/built-ins/Date/prototype/Symbol.toPrimitive/this-val-non-obj.js
var method = Date.prototype[Symbol.toPrimitive];
var descriptor = Object.getOwnPropertyDescriptor(Date.prototype, Symbol.toPrimitive);
if (typeof method !== "function") {
  throw "expected Date.prototype[Symbol.toPrimitive] to be a function";
}
if (method.length !== 1) {
  throw "expected Date.prototype[Symbol.toPrimitive] length";
}
if (method.name !== "[Symbol.toPrimitive]") {
  throw "expected Date.prototype[Symbol.toPrimitive] name";
}
if (descriptor.writable !== false) {
  throw "expected Date.prototype[Symbol.toPrimitive] to be non-writable";
}
if (descriptor.enumerable !== false) {
  throw "expected Date.prototype[Symbol.toPrimitive] to be non-enumerable";
}
if (descriptor.configurable !== true) {
  throw "expected Date.prototype[Symbol.toPrimitive] to be configurable";
}

var log = "";
var defaultLike = {
  toString: function () {
    log += "t";
    return {};
  },
  valueOf: function () {
    log += "v";
    return 5;
  },
};
if (method.call(defaultLike, "default") !== 5 || log !== "tv") {
  throw "expected default hint to try toString before valueOf";
}

var numberLog = "";
var numberLike = {
  toString: function () {
    numberLog += "t";
    return "str";
  },
  valueOf: function () {
    numberLog += "v";
    return 7;
  },
};
if (method.call(numberLike, "number") !== 7 || numberLog !== "v") {
  throw "expected number hint to try valueOf before toString";
}

var invalidHint = false;
try {
  method.call({}, "bad");
} catch (error) {
  invalidHint = error instanceof TypeError;
}
if (invalidHint !== true) {
  throw "expected invalid hint to throw TypeError";
}

var primitiveThis = false;
try {
  method.call(1, "string");
} catch (error) {
  primitiveThis = error instanceof TypeError;
}
if (primitiveThis !== true) {
  throw "expected primitive this to throw TypeError";
}
