(function () {
  var object = { first: 1, second: 2 };
  var proto = { inherited: 9 };
  var child = Object.create(proto, { own: { value: 3, enumerable: true } });
  var log = "";
  var proxy = new Proxy({ a: 1, b: 2 }, {
    ownKeys: function (target) {
      log += "|ownKeys";
      return ["a", "b"];
    },
    getOwnPropertyDescriptor: function (target, key) {
      log += "|desc:" + key;
      return Object.getOwnPropertyDescriptor(target, key);
    },
    get: function (target, key) {
      log += "|get:" + key;
      return target[key];
    }
  });
  return [
    Object.values.length,
    Object.values(object).join("|"),
    Object.values([4, 5]).join("|"),
    Object.values("ab").join("|"),
    Object.values(child).join("|"),
    Object.values(proxy).join("|"),
    log,
    Object.values(0).length
  ].join(":");
})()
