// Derived from: test/built-ins/Object/prototype/toLocaleString/primitive_this_value_getter.js
var receiver;
Object.defineProperty(Boolean.prototype, "toString", {
  get: function () {
    "use strict";
    receiver = this;
    return function () {
      return receiver === true ? "primitive receiver" : "unexpected receiver";
    };
  },
  configurable: true
});

if (Object.prototype.toLocaleString.call(true) !== "primitive receiver") {
  throw new Error("toLocaleString should call primitive prototype toString getter");
}
