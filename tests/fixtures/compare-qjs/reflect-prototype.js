(function () {
  var objectProto = { marker: 7 };
  var object = {};
  var objectSet = Reflect.setPrototypeOf(object, objectProto);
  var arrayProto = { marker: 11 };
  var array = [];
  var arraySet = Reflect.setPrototypeOf(array, arrayProto);
  var fnProto = { marker: 13 };
  function fn() {}
  var fnSet = Reflect.setPrototypeOf(fn, fnProto);
  var sealed = {};
  Object.preventExtensions(sealed);
  return [
    typeof Reflect,
    Reflect.getPrototypeOf({}) === Object.prototype,
    Reflect.getPrototypeOf([]) === Array.prototype,
    Reflect.getPrototypeOf(Object.create(null)) === null,
    objectSet,
    object.marker,
    arraySet,
    array.marker,
    fnSet,
    fn.marker,
    Reflect.setPrototypeOf(sealed, null),
    Reflect.getPrototypeOf.length,
    Reflect.setPrototypeOf.length
  ].join(":");
})()
