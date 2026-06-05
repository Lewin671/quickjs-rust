(function () {
  var first = Symbol("first");
  var second = Symbol("second");
  var object = {};
  Object.defineProperty(object, first, { value: 11, enumerable: true, configurable: true });
  Object.defineProperty(object, second, { value: 22 });
  var symbols = Object.getOwnPropertySymbols(object);
  return Object.getOwnPropertySymbols.length + ":" +
    symbols.length + ":" +
    (symbols[0] === first) + ":" +
    (symbols[1] === second) + ":" +
    Object.getOwnPropertyDescriptor(object, symbols[0]).value + ":" +
    Object.hasOwn(object, second) + ":" +
    Object.getOwnPropertyNames(object).length;
})()
