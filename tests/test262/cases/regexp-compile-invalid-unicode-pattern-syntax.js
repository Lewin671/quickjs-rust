// Derived from: test/annexB/built-ins/RegExp/prototype/compile/pattern-string-invalid-u.js
var subject = /test262/ig;

function assertInvalid(source) {
  var caught = false;
  try {
    subject.compile(source, "u");
  } catch (error) {
    caught = error instanceof SyntaxError;
  }
  if (!caught) { throw "expected SyntaxError"; }
}

assertInvalid("{");
assertInvalid("\\2");

if (subject.toString() !== new RegExp("test262", "ig").toString()) { throw; }
if (subject.test("tEsT262") !== true) { throw; }
