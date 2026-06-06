// Derived from: test/built-ins/Array/prototype/includes/call-with-boolean.js
// Derived from: test/built-ins/Array/prototype/includes/values-are-not-cached.js
if (Array.prototype.includes.call("abc", "b") !== true) {
  throw "expected includes to read string receivers";
}

var object = { length: 3, 0: "a" };
var calls = 0;
Object.defineProperty(object, "1", {
  get: function() {
    calls += 1;
    object[2] = "z";
    return "b";
  }
});

if (Array.prototype.includes.call(object, "z") !== true) {
  throw "expected includes to read mutated later indexes";
}
if (calls !== 1) {
  throw "expected includes to read each visited property once";
}

var fromIndexCalls = 0;
var fromIndex = {
  valueOf: function() {
    fromIndexCalls += 1;
    return 1;
  }
};

if ([0, 1].includes(1, fromIndex) !== true) {
  throw "expected includes to coerce fromIndex";
}
if (fromIndexCalls !== 1) {
  throw "expected includes to coerce fromIndex exactly once";
}
