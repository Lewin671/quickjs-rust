// Derived from: test/built-ins/Array/prototype/pop/throws-with-string-receiver.js
var caughtEmpty = false;
try {
  Array.prototype.pop.call("");
} catch (error) {
  caughtEmpty = error instanceof TypeError;
}
if (!caughtEmpty) {
  throw "Array.prototype.pop should throw on an empty string receiver";
}

var caughtText = false;
try {
  Array.prototype.pop.call("abc");
} catch (error) {
  caughtText = error instanceof TypeError;
}
if (!caughtText) {
  throw "Array.prototype.pop should throw on a non-empty string receiver";
}
