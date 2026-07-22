(function () {
  var prototype = [];
  prototype.marker = "array";
  Reflect.setPrototypeOf(TypeError, prototype);
  return [
    Reflect.getPrototypeOf(TypeError) === prototype,
    Object.getPrototypeOf(TypeError) === prototype,
    TypeError.__proto__ === prototype,
    TypeError.marker === "array",
    TypeError.isError === undefined,
    !("isError" in TypeError)
  ].join(":");
})()
