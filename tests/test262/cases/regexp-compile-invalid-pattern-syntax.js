// Derived from: test/annexB/built-ins/RegExp/prototype/compile/pattern-string-invalid.js
var subject = /test262/ig;

function assertInvalid(source) {
  var caught = false;
  try {
    subject.compile(source);
  } catch (error) {
    caught = error instanceof SyntaxError;
  }
  if (!caught) { throw "expected SyntaxError"; }
}

assertInvalid("?");
assertInvalid(".{2,1}");

if (subject.toString() !== new RegExp("test262", "ig").toString()) { throw; }
if (subject.test("TEsT262") !== true) { throw; }
