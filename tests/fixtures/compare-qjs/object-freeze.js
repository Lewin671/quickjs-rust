(function () {
  var object = { value: 1 };
  var array = [1];
  function fn(a) {}
  fn.value = 1;
  var same = Object.freeze(object) === object;
  Object.freeze(array);
  Object.freeze(fn);
  try { object.value = 2; } catch (error) {}
  try { array[0] = 2; } catch (error) {}
  try { array.length = 0; } catch (error) {}
  try { fn.value = 2; } catch (error) {}
  return [
    Object.freeze.length,
    Object.isFrozen.length,
    Object.isExtensible(object),
    Object.isSealed(object),
    Object.isFrozen(object),
    Object.getOwnPropertyDescriptor(object, "value").configurable,
    Object.getOwnPropertyDescriptor(object, "value").writable,
    object.value,
    same,
    Object.isFrozen(array),
    Object.getOwnPropertyDescriptor(array, "0").writable,
    array.length,
    array[0],
    Object.isFrozen(fn),
    Object.getOwnPropertyDescriptor(fn, "length").configurable,
    fn.value,
    Object.freeze(1)
  ].join(":");
})()
