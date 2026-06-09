// Derived from: test/built-ins/RegExp/prototype/exec/S15.10.6.2_A1_T5.js
// Derived from: test/built-ins/RegExp/prototype/exec/S15.10.6.2_A1_T6.js

function assertMatch(result, values, index, input) {
  if (!(result instanceof Array)) {
    throw "expected exec result array";
  }
  if (result.length !== values.length) {
    throw "unexpected match length";
  }
  if (result.index !== index) {
    throw "unexpected match index";
  }
  if (result.input !== input) {
    throw "unexpected match input";
  }
  for (var i = 0; i < values.length; i++) {
    if (result[i] !== values[i]) {
      throw "unexpected match value";
    }
  }
}

assertMatch(
  /(aa|aabaac|ba|b|c)*/.exec({
    toString: function() { return {}; },
    valueOf: function() { return "aabaac"; }
  }),
  ["aaba", "ba"],
  0,
  "aabaac"
);

assertMatch(
  /(z)((a+)?(b+)?(c))*/.exec((function() { return "zaacbbbcac"; })()),
  ["zaacbbbcac", "z", "ac", "a", undefined, "c"],
  0,
  "zaacbbbcac"
);
