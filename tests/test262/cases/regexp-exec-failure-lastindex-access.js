// Derived from: test/built-ins/RegExp/prototype/exec/failure-lastindex-access.js

var gets = 0;
var counter = {
  valueOf: function() {
    gets++;
    return 0;
  }
};

var r = /a/;
r.lastIndex = counter;

var result = r.exec("nbc");
if (result !== null) {
  throw "expected non-match";
}
if (r.lastIndex !== counter) {
  throw "expected non-global exec not to write lastIndex";
}
if (gets !== 1) {
  throw "expected exec to read lastIndex once";
}
