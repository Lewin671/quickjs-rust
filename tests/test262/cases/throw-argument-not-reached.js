// Derived from: test/language/statements/throw/S12.13_A1.js
var reached = true;

if (false) {
  throw "expected_message";
}

if (reached !== true) {
  throw;
}
