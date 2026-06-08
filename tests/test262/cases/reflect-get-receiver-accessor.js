// Derived from: test/built-ins/Reflect/get/return-value-from-receiver.js
var ownTarget = {};
var ownReceiver = {
  y: 42
};
Object.defineProperty(ownTarget, "x", {
  get: function() {
    return this.y;
  }
});
if (Reflect.get(ownTarget, "x", ownReceiver) !== 42) {
  throw "Reflect.get should call own getter with receiver";
}

var protoTarget = Object.create(ownTarget);
if (Reflect.get(protoTarget, "x", ownReceiver) !== 42) {
  throw "Reflect.get should call inherited getter with receiver";
}
