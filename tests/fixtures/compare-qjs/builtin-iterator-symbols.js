(function () {
  function attrs(object, method) {
    var descriptor = Object.getOwnPropertyDescriptor(object, Symbol.iterator);
    return (descriptor.value === object[method]) + ":" + descriptor.writable + ":" +
      descriptor.enumerable + ":" + descriptor.configurable;
  }
  var symbolDescriptor = Object.getOwnPropertyDescriptor(Symbol, "iterator");
  var arrayNext = [5][Symbol.iterator]().next();
  var mapNext = new Map([["k", 7]])[Symbol.iterator]().next();
  var setNext = new Set(["v"])[Symbol.iterator]().next();
  return typeof Symbol.iterator + ":" + symbolDescriptor.writable + ":" +
    symbolDescriptor.enumerable + ":" + symbolDescriptor.configurable + "|" +
    attrs(Array.prototype, "values") + "|" + attrs(Map.prototype, "entries") +
    "|" + attrs(Set.prototype, "values") + "|" + arrayNext.value + ":" +
    arrayNext.done + "|" + mapNext.value[0] + ":" + mapNext.value[1] + ":" +
    mapNext.done + "|" + setNext.value + ":" + setNext.done;
})()
