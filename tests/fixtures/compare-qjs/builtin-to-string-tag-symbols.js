(function () {
  function attrs(object) {
    var descriptor = Object.getOwnPropertyDescriptor(object, Symbol.toStringTag);
    return descriptor ? descriptor.value + ":" + descriptor.writable + ":" +
      descriptor.enumerable + ":" + descriptor.configurable : "undefined";
  }
  var setDeleted = delete Set.prototype[Symbol.toStringTag];
  return attrs(Symbol.prototype) + "|" + attrs(Map.prototype) + "|" +
    attrs(Set.prototype) + "|" + attrs(WeakMap.prototype) + "|" +
    attrs(WeakSet.prototype) + "|" + attrs(Promise.prototype) + "|" +
    attrs(Math) + "|" + attrs(JSON) + "|" + attrs(RegExp.prototype) + "|" +
    setDeleted + ":" + Object.prototype.toString.call(new Set());
})()
