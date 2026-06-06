(function () {
  var names = [
    "asyncDispose",
    "asyncIterator",
    "dispose",
    "hasInstance",
    "isConcatSpreadable",
    "iterator",
    "match",
    "matchAll",
    "replace",
    "search",
    "species",
    "split",
    "toPrimitive",
    "toStringTag",
    "unscopables",
  ];
  return names.map(function (name) {
    var descriptor = Object.getOwnPropertyDescriptor(Symbol, name);
    return name + ":" + typeof Symbol[name] + ":" + descriptor.writable + ":" +
      descriptor.enumerable + ":" + descriptor.configurable + ":" +
      String(Symbol[name]) + ":" + (Symbol.keyFor(Symbol[name]) === undefined);
  }).join("|");
})()
