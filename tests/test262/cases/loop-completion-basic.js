// Derived from: test/language/statements/while/cptn-abrupt-empty.js
// Derived from: test/language/statements/for-of/cptn-decl-abrupt-empty.js
// Derived from: test/language/statements/for-of/cptn-expr-abrupt-empty.js
// Derived from: test/language/statements/for-of/cptn-decl-itr.js
// Derived from: test/language/statements/for-of/cptn-expr-itr.js

if (eval("1; while (true) { break; }") !== undefined) {
  throw "expected empty while break completion to remain undefined";
}
if (eval("2; while (true) { 3; break; }") !== 3) {
  throw "expected while break to preserve prior statement completion";
}
if (eval("4; for (var a of [0]) { break; }") !== undefined) {
  throw "expected empty for-of var break completion to remain undefined";
}
if (eval("5; for (var b of [0]) { 6; break; }") !== 6) {
  throw "expected for-of var break to preserve prior statement completion";
}

var target;
if (eval("7; for (target of [0]) { break; }") !== undefined) {
  throw "expected empty for-of assignment break completion to remain undefined";
}
if (eval("8; for (target of [0]) { 9; break; }") !== 9) {
  throw "expected for-of assignment break to preserve prior statement completion";
}

if (eval("10; for (var c of [0]) { }") !== undefined) {
  throw "expected empty for-of body completion to remain undefined";
}
if (eval("11; for (var d of [0]) { 12; }") !== 12) {
  throw "expected for-of iteration completion to use body value";
}
if (eval("13; for (target of [0]) { }") !== undefined) {
  throw "expected empty assignment-head for-of body completion to remain undefined";
}
if (eval("14; for (target of [0]) { 15; }") !== 15) {
  throw "expected assignment-head for-of iteration completion to use body value";
}

if (eval("16; for (var e of [0, 1]) { 17; continue; }") !== 17) {
  throw "expected for-of continue to preserve prior statement completion";
}
