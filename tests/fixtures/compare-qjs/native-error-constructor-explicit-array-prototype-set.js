(function () {
  var hit = 0;
  var prototype = [];
  Object.defineProperty(prototype, "x", {
    set: function (value) { hit = value; },
    configurable: true
  });
  Reflect.setPrototypeOf(TypeError, prototype);
  TypeError.x = 7;
  return hit + ":" + Object.prototype.hasOwnProperty.call(TypeError, "x");
})()
