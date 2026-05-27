// Derived from: test/language/statements/function/S13_A18.js
function make(value) {
  return function() {
    return value;
  };
}

var get = make("closed");
if (get() !== "closed") { throw; }
