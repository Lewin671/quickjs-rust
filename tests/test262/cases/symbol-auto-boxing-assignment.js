// Derived from: test/built-ins/Symbol/auto-boxing-non-strict.js
// Derived from: test/built-ins/Symbol/auto-boxing-strict.js
var sym = Symbol("66");

sym.a = 0;
if (sym.a !== undefined) {
  throw new Error("expected Symbol primitive string property assignment to be ignored");
}

sym["a" + "b"] = 0;
if (sym["a" + "b"] !== undefined) {
  throw new Error("expected Symbol primitive computed property assignment to be ignored");
}

sym[62] = 0;
if (sym[62] !== undefined) {
  throw new Error("expected Symbol primitive numeric property assignment to be ignored");
}

(function () {
  "use strict";
  try {
    Symbol("66").a = 0;
    throw new Error("expected strict Symbol primitive assignment to throw");
  } catch (error) {
    if (!(error instanceof TypeError)) {
      throw error;
    }
  }
}());
