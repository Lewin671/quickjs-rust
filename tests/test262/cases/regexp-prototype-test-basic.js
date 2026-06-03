// Derived from: test/built-ins/RegExp/prototype/test/S15.10.6.3_A1_T1.js
if (/test/.test("a test value") !== true) {
  throw "RegExp.prototype.test should return true for a match";
}

if (/missing/.test("a test value") !== false) {
  throw "RegExp.prototype.test should return false for a miss";
}

var re = /34/g;
if (re.test("343443444") !== true) {
  throw "global RegExp.prototype.test should return true for first match";
}
if (re.lastIndex !== 2) {
  throw "global RegExp.prototype.test should update lastIndex";
}
