// Derived from: test/built-ins/String/prototype/split/call-split-undefined-instance-is-string-hello.js
var split = "hello".split(undefined);
if (split.length !== 1 || split[0] !== "hello") {
  throw "expected undefined split separator to return the full string";
}
if ("hello".split(undefined, 0).length !== 0) {
  throw "expected undefined separator with zero limit to return an empty array";
}
