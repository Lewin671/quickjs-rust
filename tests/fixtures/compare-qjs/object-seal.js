(function () {
  var object = { value: 1 };
  var array = [1];
  function fn() {}
  var same = Object.seal(object) === object;
  Object.seal(array);
  Object.seal(fn);
  object.value = 2;
  return [
    Object.seal.length,
    Object.isSealed.length,
    Object.isSealed({}),
    same,
    Object.isExtensible(object),
    Object.isSealed(object),
    Object.getOwnPropertyDescriptor(object, "value").configurable,
    object.value,
    Object.isSealed(array),
    Object.getOwnPropertyDescriptor(array, "0").configurable,
    Object.isSealed(fn),
    Object.getOwnPropertyDescriptor(fn, "length").configurable,
    Object.isSealed(1),
    Object.seal(1)
  ].join(":");
})()
