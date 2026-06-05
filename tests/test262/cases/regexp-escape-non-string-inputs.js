// Derived from: test/built-ins/RegExp/escape/non-string-inputs.js

function assertThrows(value) {
  var caught = false;
  try {
    RegExp.escape(value);
  } catch (error) {
    caught = error instanceof TypeError;
  }
  if (!caught) {
    throw "RegExp.escape should reject non-string input";
  }
}

assertThrows(123);
assertThrows({});
assertThrows([]);
assertThrows(null);
assertThrows(undefined);
