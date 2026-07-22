(function () {
  var target = {};
  Object.defineProperty(target, "x", {
    value: 1,
    enumerable: false,
    configurable: true
  });
  var proxy = new Proxy(target, {
    ownKeys: function () { return ["x"]; },
    getOwnPropertyDescriptor: function () {
      return { value: 1, enumerable: true, configurable: true };
    }
  });
  var bridge = [];
  Object.setPrototypeOf(bridge, proxy);
  var object = Object.create(bridge);
  var ordinarySeen = "";
  for (var key in object) {
    ordinarySeen += key;
  }
  var functionValue = function () {};
  Object.setPrototypeOf(functionValue, bridge);
  var functionSeen = "";
  for (var functionKey in functionValue) {
    functionSeen += functionKey;
  }
  return ordinarySeen + ":" + functionSeen;
})()
