(function () {
  var object = { value: 1 };
  Object.defineProperty(object, "hidden", { value: 2 });
  var descriptors = Object.getOwnPropertyDescriptors(object);
  var child = Object.create({ inherited: 1 }, { own: { value: 3, enumerable: true } });
  var childDescriptors = Object.getOwnPropertyDescriptors(child);
  var stringDescriptors = Object.getOwnPropertyDescriptors("ab");
  return [
    Object.getOwnPropertyDescriptors.length,
    Object.getPrototypeOf(Object.getOwnPropertyDescriptors({})) === Object.prototype,
    descriptors.value.value,
    descriptors.value.enumerable,
    descriptors.hidden.value,
    descriptors.hidden.enumerable,
    Object.keys(childDescriptors).join("|"),
    stringDescriptors.length.value,
    stringDescriptors[0].value,
    stringDescriptors[0].writable,
    stringDescriptors[0].configurable,
    Object.keys(Object.getOwnPropertyDescriptors(0)).length
  ].join(":");
})()
