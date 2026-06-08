// Derived from: test/annexB/built-ins/RegExp/prototype/compile/pattern-string-u.js
var subject = /original value/ig;

subject.compile("[\ud834\udf06]", "u");

if (subject.source !== new RegExp("[\ud834\udf06]", "u").source) {
  throw "expected compile to replace source";
}
if (subject.test("original value") !== false) {
  throw "expected old source not to match";
}
if (subject.test("\ud834") !== false) {
  throw "expected high surrogate alone not to match";
}
if (subject.test("\udf06") !== false) {
  throw "expected low surrogate alone not to match";
}
if (subject.test("\ud834\udf06") !== true) {
  throw "expected surrogate pair to match";
}
