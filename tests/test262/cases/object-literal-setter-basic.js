// Derived from: test/language/expressions/object/setter-prop-desc.js
var seen = 0;
var object = {
  set value(next) {
    seen = next;
  }
};

object.value = 42;
if (seen !== 42) {
  throw "expected object literal setter to be called";
}
