(function () {
  function invariant(operation) {
    var calls = 0;
    var target = {};
    Object.defineProperty(target, "x", {
      value: 1,
      writable: false,
      configurable: false
    });
    var inner = new Proxy(target, {
      getOwnPropertyDescriptor: function (object, key) {
        calls += 1;
        return Reflect.getOwnPropertyDescriptor(object, key);
      }
    });
    var handler = {};
    handler[operation] = function () {
      return operation === "get" ? 2 : operation === "has" ? false : true;
    };
    var outer = new Proxy(inner, handler);
    var threw = false;
    try {
      if (operation === "get") outer.x;
      else if (operation === "has") "x" in outer;
      else if (operation === "set") Reflect.set(outer, "x", 2);
      else Reflect.deleteProperty(outer, "x");
    } catch (error) {
      threw = error instanceof TypeError;
    }
    return threw + ":" + calls;
  }

  function ownKeysOrder() {
    var log = [];
    var target = { x: 1 };
    var inner = new Proxy(target, {
      isExtensible: function (object) {
        log.push("e");
        return Reflect.isExtensible(object);
      },
      ownKeys: function (object) {
        log.push("k");
        return Reflect.ownKeys(object);
      },
      getOwnPropertyDescriptor: function (object, key) {
        log.push("d");
        return Reflect.getOwnPropertyDescriptor(object, key);
      }
    });
    Reflect.ownKeys(new Proxy(inner, { ownKeys: function () { return []; } }));
    return log.join(",");
  }

  return [
    invariant("get"),
    invariant("has"),
    invariant("set"),
    invariant("deleteProperty"),
    ownKeysOrder()
  ].join("|");
})()
