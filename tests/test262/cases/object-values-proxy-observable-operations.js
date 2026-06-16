// Derived from: test/built-ins/Object/values/observable-operations.js
var log = "";
var proxy = new Proxy({ a: 1, b: 2, c: 3 }, {
  ownKeys: function(target) {
    log += "|ownKeys";
    return Object.getOwnPropertyNames(target);
  },
  getOwnPropertyDescriptor: function(target, key) {
    log += "|getOwnPropertyDescriptor:" + key;
    return Object.getOwnPropertyDescriptor(target, key);
  },
  get: function(target, key) {
    log += "|get:" + key;
    return target[key];
  }
});

var values = Object.values(proxy);
if (values.length !== 3 || values[0] !== 1 || values[1] !== 2 || values[2] !== 3) {
  throw "Object.values should return enumerable proxy values";
}
if (log !== "|ownKeys|getOwnPropertyDescriptor:a|get:a|getOwnPropertyDescriptor:b|get:b|getOwnPropertyDescriptor:c|get:c") {
  throw "Object.values should observe proxy ownKeys, getOwnPropertyDescriptor, and get traps in key order";
}
