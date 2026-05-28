// Derived from: test/built-ins/Object/prototype/toLocaleString/S15.2.4.3_A1.js
if (Object.prototype.toLocaleString() !== "[object Object]") {
  throw new Error("default Object.prototype.toLocaleString should call toString");
}

var object = {
  toString: function () {
    return "custom";
  }
};
if (object.toLocaleString() !== "custom") {
  throw new Error("toLocaleString should call this.toString");
}
