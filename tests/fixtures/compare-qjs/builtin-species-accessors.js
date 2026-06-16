(function () {
  function describe(ctor) {
    var desc = Object.getOwnPropertyDescriptor(ctor, Symbol.species);
    var receiver = {};
    return (desc.get.call(receiver) === receiver) + ":" +
      (desc.set === undefined) + ":" +
      desc.enumerable + ":" +
      desc.configurable + ":" +
      desc.get.name + ":" +
      desc.get.length;
  }
  return describe(Array) + "|" + describe(Map) + "|" + describe(Promise) + "|" +
    describe(RegExp) + "|" + describe(Set);
})()
