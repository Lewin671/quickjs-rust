// Derived from: test/annexB/built-ins/RegExp/prototype/compile/pattern-string.js
var subject = /original value/ig;
var result = subject.compile("new value");

if (result !== subject) {
  throw "expected compile to return receiver";
}
if (subject.source !== new RegExp("new value").source) {
  throw "expected compile to replace source";
}
if (subject.flags !== "") {
  throw "expected omitted flags to replace flags with empty string";
}
if (subject.test("original value") !== false) {
  throw "expected old source not to match";
}
if (subject.test("new value") !== true) {
  throw "expected new source to match";
}
