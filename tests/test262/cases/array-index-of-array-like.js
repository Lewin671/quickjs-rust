// Derived from: test/built-ins/Array/prototype/indexOf/15.4.4.14-1-3.js
// Derived from: test/built-ins/Array/prototype/lastIndexOf/15.4.4.15-1-3.js
if (Array.prototype.indexOf.call("abc", "b") !== 1) {
  throw "expected indexOf to read string receivers";
}
if (Array.prototype.lastIndexOf.call("abc", "b") !== 1) {
  throw "expected lastIndexOf to read string receivers";
}

var object = { length: 3, 2: "x" };
if (Array.prototype.indexOf.call(object, "x") !== 2) {
  throw "expected indexOf to read array-like objects";
}
if (Array.prototype.lastIndexOf.call(object, "x") !== 2) {
  throw "expected lastIndexOf to read array-like objects";
}

if ([, undefined].indexOf(undefined) !== 1) {
  throw "expected indexOf to skip holes";
}
if ([undefined, ,].lastIndexOf(undefined) !== 0) {
  throw "expected lastIndexOf to skip holes";
}

var indexOfCalls = 0;
var indexOfFromIndex = {
  valueOf: function() {
    indexOfCalls += 1;
    return 1;
  }
};
if ([0, 1].indexOf(1, indexOfFromIndex) !== 1) {
  throw "expected indexOf to coerce fromIndex";
}
if (indexOfCalls !== 1) {
  throw "expected indexOf to coerce fromIndex exactly once";
}

var lastIndexOfCalls = 0;
var lastIndexOfFromIndex = {
  valueOf: function() {
    lastIndexOfCalls += 1;
    return 1;
  }
};
if ([0, 1, 2].lastIndexOf(1, lastIndexOfFromIndex) !== 1) {
  throw "expected lastIndexOf to coerce fromIndex";
}
if (lastIndexOfCalls !== 1) {
  throw "expected lastIndexOf to coerce fromIndex exactly once";
}
