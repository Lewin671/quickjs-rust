// Derived from: test/built-ins/Proxy/set/call-parameters-prototype-index.js
var original = Object.getPrototypeOf(Function.prototype);
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

Object.setPrototypeOf(Function.prototype, proxy);
function link() {}
var bridge = Object.create(link);
var array = [];
Object.setPrototypeOf(array, bridge);
array[0] = 17;
Object.setPrototypeOf(Function.prototype, original);

if (seenTarget !== original || seenKey !== "0" || seenValue !== 17 || seenReceiver !== array) {
  throw "expected assignment to reach Proxy after the implicit Function.prototype";
}
if (Object.prototype.hasOwnProperty.call(array, "0")) {
  throw "expected the inherited Proxy set trap to handle the assignment";
}
