// Derived from: test/language/rest-parameters/rest-index.js
if ((function(...args) { return args.length; })(1, 2, 3, 4, 5) !== 5) {
  throw "expected rest index 0";
}
if ((function(a, ...args) { return args.length; })(1, 2, 3, 4, 5) !== 4) {
  throw "expected rest index 1";
}
if ((function(a, b, ...args) { return args.length; })(1, 2, 3, 4, 5) !== 3) {
  throw "expected rest index 2";
}
if ((function(a, b, c, ...args) { return args.length; })(1, 2, 3, 4, 5) !== 2) {
  throw "expected rest index 3";
}
if ((function(a, b, c, d, ...args) { return args.length; })(1, 2, 3, 4, 5) !== 1) {
  throw "expected rest index 4";
}
if ((function(a, b, c, d, e, ...args) { return args.length; })(1, 2, 3, 4, 5) !== 0) {
  throw "expected rest index after all arguments";
}
