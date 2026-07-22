(function () {
  var prototype = [];
  var functionValue = function () {};
  var getCalls = 0;
  var receiverOk = false;
  Object.setPrototypeOf(functionValue, prototype);
  Object.setPrototypeOf(prototype, new Proxy({}, {
    get: function (target, key, receiver) {
      getCalls += 1;
      receiverOk = receiver === functionValue;
      return key === "answer" ? 9 : Reflect.get(target, key, receiver);
    }
  }));
  var typedArray = new Uint8Array([7]);
  var typedPrototype = [];
  var typedFunction = function () {};
  Object.setPrototypeOf(typedPrototype, typedArray);
  Object.setPrototypeOf(typedFunction, typedPrototype);

  function targetFor(key) {
    var target = {};
    Object.defineProperty(target, key, { value: 0, writable: true });
    return target;
  }

  function getOwnThrows(key) {
    var inner = new Proxy({}, {
      getOwnPropertyDescriptor: function () { throw 41; }
    });
    var outer = new Proxy(inner, {
      getOwnPropertyDescriptor: function () { return undefined; }
    });
    try {
      Reflect.set(targetFor(key), key, 1, outer);
    } catch (error) {
      return error === 41;
    }
    return false;
  }

  function invalidResultSkipsTarget(key) {
    var reads = 0;
    var inner = new Proxy({}, {
      getOwnPropertyDescriptor: function () { reads += 1; throw 44; }
    });
    var outer = new Proxy(inner, {
      getOwnPropertyDescriptor: function () { return 1; }
    });
    try {
      Object.getOwnPropertyDescriptor(outer, key);
    } catch (error) {
      return error instanceof TypeError && reads === 0;
    }
    return false;
  }

  function defineInvariantThrows(key) {
    var reads = 0;
    var inner = new Proxy({}, {
      getOwnPropertyDescriptor: function () {
        reads += 1;
        if (reads === 2) throw 43;
        return undefined;
      }
    });
    var outer = new Proxy(inner, {
      getOwnPropertyDescriptor: function () { return undefined; },
      defineProperty: function () { return true; }
    });
    try {
      Reflect.set(targetFor(key), key, 1, outer);
    } catch (error) {
      return error === 43 && reads === 2;
    }
    return false;
  }

  var symbol = Symbol("nested");
  var seen = 0;
  Object.defineProperty(Error, "0", {
    set: function (value) { seen = value; },
    configurable: true
  });
  var array = [];
  Object.setPrototypeOf(array, TypeError);
  array[0] = 7;

  function objectPrototypeSet(method, bridgeLength, value, args) {
    var log = "";
    var bridge = new Array(bridgeLength);
    var receiver;
    Object.setPrototypeOf(bridge, new Proxy({}, {
      set: function (target, key, nextValue, actualReceiver) {
        log += key + ":" + nextValue + ":" + (actualReceiver === receiver) + "|";
        return true;
      }
    }));
    receiver = Object.create(bridge);
    if (value !== undefined) {
      Object.defineProperty(receiver, "1", {
        value: value,
        writable: true,
        enumerable: true,
        configurable: true
      });
      receiver.length = 2;
    }
    var result = method.apply(receiver, args);
    var outcome = method === Array.prototype.push ? result : result === receiver;
    return log + ":" + outcome + ":"
      + Object.prototype.hasOwnProperty.call(receiver, "0");
  }

  return [
    functionValue.answer,
    getCalls,
    receiverOk,
    typedFunction[0],
    String(typedFunction[1]),
    getOwnThrows("string"),
    getOwnThrows(symbol),
    invalidResultSkipsTarget("string"),
    invalidResultSkipsTarget(symbol),
    defineInvariantThrows("string"),
    defineInvariantThrows(symbol),
    seen,
    array.hasOwnProperty("0"),
    String(array[0]),
    objectPrototypeSet(Array.prototype.push, 0, undefined, [7]),
    objectPrototypeSet(Array.prototype.fill, 1, undefined, [8]),
    objectPrototypeSet(Array.prototype.copyWithin, 0, 5, [0, 1, 2])
  ].join(":");
})()
