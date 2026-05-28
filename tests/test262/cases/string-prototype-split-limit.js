// Derived from: test/built-ins/String/prototype/split/call-split-l-2-instance-is-string-hello.js
var split = "hello".split("l", 2);
if (split.length !== 2 || split[0] !== "he" || split[1] !== "") {
  throw "expected split limit to cap returned elements";
}
if ("hello".split("l", 0).length !== 0) {
  throw "expected zero split limit to return an empty array";
}
