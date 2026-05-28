// Derived from: test/built-ins/String/prototype/split/call-split-l-instance-is-string-hello.js
var split = "hello".split("l");
if (split.constructor !== Array) {
  throw "expected split to return an Array";
}
if (split.length !== 3 || split[0] !== "he" || split[1] !== "" || split[2] !== "o") {
  throw "expected split to divide the string around each separator";
}
