// Derived from: test/built-ins/Object/keys/proxy-keys.js
var log = "";
var trapKeys = {
  get length() {
    log += "|length";
    return 3;
  },
  get 0() {
    log += "|key0";
    return "a";
  },
  get 1() {
    log += "|key1";
    return Symbol("skip");
  },
  get 2() {
    log += "|key2";
    return "b";
  }
};
var proxy = new Proxy({}, {
  ownKeys: function() {
    log += "|ownKeys";
    return trapKeys;
  },
  getOwnPropertyDescriptor: function(target, key) {
    log += "|desc:" + key;
    return { value: key, enumerable: key === "a", configurable: true };
  }
});

var keys = Object.keys(proxy);
if (keys.length !== 1 || keys[0] !== "a") {
  throw "Object.keys should include only enumerable string keys from proxy ownKeys";
}
if (log !== "|ownKeys|length|key0|key1|key2|desc:a|desc:b") {
  throw "Object.keys should consume array-like ownKeys results before descriptor checks";
}
