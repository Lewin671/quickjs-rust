// Derived from: test/built-ins/Reflect/set/set-value-on-accessor-descriptor-with-receiver.js
var count = 0;
var seenValue;
var seenReceiver;
var receiver = {};
var target = {};
Object.defineProperty(target, "value", {
  set: function(value) {
    count = count + 1;
    seenValue = value;
    seenReceiver = this;
  }
});

if (Reflect.set(target, "value", 42, receiver) !== true) {
  throw "expected Reflect.set to return true for accessor setter";
}
if (count !== 1 || seenValue !== 42 || seenReceiver !== receiver) {
  throw "expected Reflect.set to call accessor setter with receiver";
}
