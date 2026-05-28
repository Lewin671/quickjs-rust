(function () {
  var object = {
    toString: function () {
      return "custom";
    }
  };
  return [
    Object.prototype.toLocaleString.length,
    Object.prototype.toLocaleString(),
    object.toLocaleString(),
    Object.prototype.propertyIsEnumerable("toLocaleString"),
    Object.prototype.hasOwnProperty("toLocaleString")
  ].join(":");
})()
