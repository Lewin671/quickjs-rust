// Derived from: test/built-ins/RegExp/prototype/exec/S15.10.6.2_A1_T1.js
// Derived from: test/built-ins/RegExp/prototype/exec/S15.10.6.2_A1_T10.js
// Derived from: test/built-ins/RegExp/prototype/exec/S15.10.6.2_A1_T11.js
// Derived from: test/built-ins/RegExp/prototype/exec/S15.10.6.2_A1_T14.js
// Derived from: test/built-ins/RegExp/prototype/exec/S15.10.6.2_A1_T17.js
// Derived from: test/built-ins/RegExp/prototype/exec/S15.10.6.2_A1_T18.js

function assertMatch(result, value, index, input) {
  if (!(result instanceof Array)) {
    throw "expected exec result array";
  }
  if (result.length !== 1) {
    throw "expected one match element";
  }
  if (result[0] !== value) {
    throw "expected matched value";
  }
  if (result.index !== index) {
    throw "expected match index";
  }
  if (result.input !== input) {
    throw "expected match input";
  }
}

assertMatch(/1|12/.exec("123"), "1", 0, "123");
assertMatch(/1|12/.exec(1.01), "1", 0, "1.01");
assertMatch(/2|12/.exec(new Number(1.012)), "12", 3, "1.012");
assertMatch(/AL|se/.exec(new Boolean(false)), "se", 3, "false");
assertMatch(/ll|l/.exec(null), "ll", 2, "null");
assertMatch(/nd|ne/.exec(undefined), "nd", 1, "undefined");
