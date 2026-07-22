(function () {
  var sloppy = [,];
  Object.preventExtensions(sloppy);
  var sloppyResult = Function("array", "return (array[0] = 7)")(sloppy);
  var strict = [,];
  Object.preventExtensions(strict);
  var strictThrew = false;
  try {
    Function("array", "\"use strict\"; return (array[0] = 7)")(strict);
  } catch (error) {
    strictThrew = error instanceof TypeError;
  }
  var lengthArray = [];
  Object.preventExtensions(lengthArray);
  var lengthResult = Reflect.set(lengthArray, "length", 3);

  var key = Symbol("key");
  var target = {};
  Object.defineProperty(target, key, { value: 1, writable: true });
  var backing = {};
  Object.defineProperty(backing, key, {
    value: 2,
    writable: true,
    configurable: true
  });
  var updateLog = [];
  var updateReceiver = new Proxy(backing, {
    getOwnPropertyDescriptor: function (object, property) {
      updateLog.push("get:" + (property === key));
      return Reflect.getOwnPropertyDescriptor(object, property);
    },
    defineProperty: function (object, property, descriptor) {
      updateLog.push(
        "define:" + (property === key) + ":" + Object.keys(descriptor).join(",")
      );
      return Reflect.defineProperty(object, property, descriptor);
    }
  });
  var updateResult = Reflect.set(target, key, 11, updateReceiver);

  var created = {};
  var createLog = [];
  var createReceiver = new Proxy(created, {
    getOwnPropertyDescriptor: function (object, property) {
      createLog.push("get:" + (property === key));
      return Reflect.getOwnPropertyDescriptor(object, property);
    },
    defineProperty: function (object, property, descriptor) {
      createLog.push(
        "define:" + (property === key) + ":" + Object.keys(descriptor).join(",")
      );
      return Reflect.defineProperty(object, property, descriptor);
    }
  });
  var createResult = Reflect.set(target, key, 13, createReceiver);

  var getThrew = false;
  try {
    Reflect.set(target, key, 17, new Proxy({}, {
      getOwnPropertyDescriptor: function () { throw 41; }
    }));
  } catch (error) {
    getThrew = error === 41;
  }
  var defineThrew = false;
  try {
    Reflect.set(target, key, 19, new Proxy({}, {
      getOwnPropertyDescriptor: function () { return undefined; },
      defineProperty: function () { throw 43; }
    }));
  } catch (error) {
    defineThrew = error === 43;
  }

  return [
    sloppyResult,
    Object.prototype.hasOwnProperty.call(sloppy, "0"),
    String(sloppy[0]),
    strictThrew,
    Object.prototype.hasOwnProperty.call(strict, "0"),
    String(strict[0]),
    lengthResult,
    lengthArray.length,
    Object.keys(lengthArray).length,
    updateResult,
    target[key],
    backing[key],
    updateLog.join("|"),
    createResult,
    created[key],
    createLog.join("|"),
    getThrew,
    defineThrew
  ].join(":");
})()
