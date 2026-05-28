(function () {
  var object = { value: 1 };
  var array = [1];
  function fn() {}
  var same = Object.preventExtensions(object) === object;
  var extensible = Object.isExtensible(object);
  Object.preventExtensions(array);
  Object.preventExtensions(fn);
  object.value = 3;
  return [
    Object.isExtensible({}),
    Object.isExtensible([]),
    Object.isExtensible(function () {}),
    same,
    extensible,
    Object.isExtensible(array),
    Object.isExtensible(fn),
    object.value,
    Object.isExtensible(1),
    Object.preventExtensions(1)
  ].join(":");
})()
