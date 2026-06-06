// Derived from: test/language/statements/const/syntax/const-outer-inner-let-bindings.js
var seen = "";

for (var i = 0; i < 3; i = i + 1) {
  const value = "inner" + i;
  seen = seen + value;
}

if (seen !== "inner0inner1inner2") {
  throw new Error("expected const declaration to initialize per loop body execution");
}
