(function () {
  var object = { first: 1, second: 2 };
  var proto = { inherited: 9 };
  var child = Object.create(proto, { own: { value: 3, enumerable: true } });
  var objectEntries = Object.entries(object);
  var arrayEntries = Object.entries([4, 5]);
  var stringEntries = Object.entries("ab");
  var childEntries = Object.entries(child);
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
  var proxyEntries = Object.entries(proxy);
  return [
    Object.entries.length,
    objectEntries[0][0] + "=" + objectEntries[0][1],
    objectEntries[1][0] + "=" + objectEntries[1][1],
    arrayEntries[0][0] + "=" + arrayEntries[0][1],
    arrayEntries[1][0] + "=" + arrayEntries[1][1],
    stringEntries[0][0] + "=" + stringEntries[0][1],
    stringEntries[1][0] + "=" + stringEntries[1][1],
    childEntries.length + ":" + childEntries[0][0] + "=" + childEntries[0][1],
    proxyEntries[0][0] + "=" + proxyEntries[0][1],
    proxyEntries[1][0] + "=" + proxyEntries[1][1],
    log,
    Object.entries(0).length
  ].join("|");
})()
