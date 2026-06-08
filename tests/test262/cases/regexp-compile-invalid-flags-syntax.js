// Derived from: test/annexB/built-ins/RegExp/prototype/compile/flags-string-invalid.js
var subject = /abcd/ig;

function assertInvalid(flags) {
  var caught = false;
  try {
    subject.compile("", flags);
  } catch (error) {
    caught = error instanceof SyntaxError;
  }
  if (!caught) { throw "expected SyntaxError"; }
}

assertInvalid("igi");
assertInvalid("gI");
assertInvalid("w");

if (subject.toString() !== new RegExp("abcd", "ig").toString()) { throw; }
if (subject.test("AbCD") !== true) { throw; }
