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
  var defined = {};
  var defineResult = Reflect.defineProperty(defined, "value", {
    value: 19,
    enumerable: true,
    writable: true,
    configurable: true
  });
  var hidden = {};
  var hiddenResult = Reflect.defineProperty(hidden, "hidden", { value: 23 });
  var blocked = {};
  Object.preventExtensions(blocked);
  var blockedResult = Reflect.defineProperty(blocked, "value", { value: 1 });
  var deleted = { value: 31 };
  var deleteResult = Reflect.deleteProperty(deleted, "value");
  var fixedDelete = {};
  Object.defineProperty(fixedDelete, "fixed", { value: 1 });
  var fixedDeleteResult = Reflect.deleteProperty(fixedDelete, "fixed");
  function deleteFn() {}
  deleteFn.value = 37;
  var functionDeleteResult = Reflect.deleteProperty(deleteFn, "value");
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
    Reflect.has(object, "marker"),
    Reflect.has(Object.create({ inherited: 1 }), "inherited"),
    Reflect.has(array, "marker"),
    Reflect.has(fn, "marker"),
    defineResult,
    defined.value,
    Object.keys(defined).join("|"),
    hiddenResult,
    Object.keys(hidden).length,
    hidden.hidden,
    blockedResult,
    deleteResult,
    Reflect.has(deleted, "value"),
    fixedDeleteResult,
    functionDeleteResult,
    Reflect.has(deleteFn, "value"),
    Reflect.getOwnPropertyDescriptor({ value: 17 }, "value").value,
    Reflect.getOwnPropertyDescriptor([1, 2], "length").enumerable,
    Reflect.ownKeys({ a: 1, b: 2 }).join("|"),
    (function () {
      var o = {};
      Object.defineProperty(o, "hidden", { value: 1 });
      o.shown = 2;
      return Reflect.ownKeys(o).join("|");
    })(),
    Reflect.setPrototypeOf(sealed, null),
    Reflect.getOwnPropertyDescriptor.length,
    Reflect.defineProperty.length,
    Reflect.deleteProperty.length,
    Reflect.has.length,
    Reflect.ownKeys.length,
    Reflect.getPrototypeOf.length,
    Reflect.setPrototypeOf.length
  ].join(":");
})()
