// Derived from: test/built-ins/RegExp/prototype/exec/success-lastindex-access.js

var gets = 0;
var counter = {
  valueOf: function() {
    gets++;
    return 0;
  }
};

var r = /./;
r.lastIndex = counter;

var result = r.exec("abc");
if (result === null) {
  throw "expected match";
}
if (result.length !== 1 || result[0] !== "a") {
  throw "expected first character match";
}
if (r.lastIndex !== counter) {
  throw "expected non-global exec not to write lastIndex";
}
if (gets !== 1) {
  throw "expected exec to read lastIndex once";
}
