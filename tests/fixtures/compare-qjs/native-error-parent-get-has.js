(function () {
  var key = Symbol("key");
  Error[key] = 4;
  Reflect.setPrototypeOf(Error, new Proxy({ x: 2 }, {}));
  return TypeError[key] + ":" + (key in TypeError) + ":"
    + TypeError.x + ":" + ("x" in TypeError);
})()
