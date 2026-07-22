// Derived from: test/built-ins/Proxy/set/call-parameters-prototype-index.js
var original = Object.getPrototypeOf(Array.prototype);
var seenTarget;
var seenKey;
var seenValue;
var seenReceiver;
var proxy = new Proxy(original, {
  set: function(target, key, value, receiver) {
    seenTarget = target;
    seenKey = key;
    seenValue = value;
    seenReceiver = receiver;
    return true;
  }
});

Object.setPrototypeOf(Array.prototype, proxy);
var array = [];
array[0] = 1;
Object.setPrototypeOf(Array.prototype, original);

if (seenTarget !== original || seenKey !== "0" || seenValue !== 1 || seenReceiver !== array) {
  throw "expected indexed assignment to follow the complete Array prototype chain";
}
if (array.hasOwnProperty("0")) {
  throw "expected the inherited proxy set trap to handle the assignment";
}
