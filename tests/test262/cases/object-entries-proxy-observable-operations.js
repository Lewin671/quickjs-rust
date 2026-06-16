// Derived from: test/built-ins/Object/entries/observable-operations.js
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

var entries = Object.entries(proxy);
if (entries.length !== 3 || entries[0][0] !== "a" || entries[0][1] !== 1) {
  throw "Object.entries should return enumerable proxy entries";
}
if (log !== "|ownKeys|getOwnPropertyDescriptor:a|get:a|getOwnPropertyDescriptor:b|get:b|getOwnPropertyDescriptor:c|get:c") {
  throw "Object.entries should observe proxy ownKeys, getOwnPropertyDescriptor, and get traps in key order";
}
