(function () {
  var arrayProto = { arrayMarker: 11 };
  var array = [];
  var arraySame = Object.setPrototypeOf(array, arrayProto) === array;
  var arrayMarker = array.arrayMarker;
  var arrayPrototype = Object.getPrototypeOf(array) === arrayProto;
  Object.setPrototypeOf(array, null);
  var arrayNull = Object.getPrototypeOf(array) === null;

  var functionProto = { functionMarker: 13 };
  function target() {}
  var functionSame = Object.setPrototypeOf(target, functionProto) === target;
  var functionMarker = target.functionMarker;
  var functionPrototype = Object.getPrototypeOf(target) === functionProto;
  Object.setPrototypeOf(target, null);
  var functionNull = Object.getPrototypeOf(target) === null;

  return [
    arraySame,
    arrayMarker,
    arrayPrototype,
    arrayNull,
    functionSame,
    functionMarker,
    functionPrototype,
    functionNull
  ].join(":");
})()
