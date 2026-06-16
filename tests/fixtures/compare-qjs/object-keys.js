(function () {
  var log = "";
  var trapKeys = {
    get length() {
      log += "|length";
      return 2;
    },
    get 0() {
      log += "|key0";
      return "a";
    },
    get 1() {
      log += "|key1";
      return "b";
    }
  };
  var proxy = new Proxy({}, {
    ownKeys: function () {
      log += "|ownKeys";
      return trapKeys;
    },
    getOwnPropertyDescriptor: function (target, key) {
      log += "|desc:" + key;
      return { value: key, enumerable: key === "a", configurable: true };
    }
  });
  return [
    Object.keys({ value: 1 })[0],
    Object.keys([1, 2]).length,
    Object.keys(Object.create({ inherited: 1 })).length,
    Object.keys(proxy).join(","),
    log
  ].join(":");
})()
