// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-594.js

var foo = function() {};
var data = "data";
Object.defineProperty(Function.prototype, "prop", {
  get: function() {
    return data;
  },
  set: function(value) {
    data = value;
  },
  enumerable: true,
  configurable: true,
});

var obj = foo.bind({});
obj.prop = "overrideData";
var hasOwn = obj.hasOwnProperty("prop");
var observed = obj.prop;
delete Function.prototype.prop;

if (hasOwn || observed !== "overrideData" || data !== "overrideData") {
  throw "expected inherited Function.prototype setter to handle assignment";
}
