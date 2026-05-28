(function () {
  var proto = { value: 7 };
  var object = {};
  var same = Object.setPrototypeOf(object, proto) === object;
  var inherited = object.value;
  var prototypeIsProto = Object.getPrototypeOf(object) === proto;
  Object.setPrototypeOf(object, null);
  var prototypeIsNull = Object.getPrototypeOf(object) === null;
  var primitive = Object.setPrototypeOf(1, null);
  return [
    Object.setPrototypeOf.length,
    same,
    inherited,
    prototypeIsProto,
    prototypeIsNull,
    primitive
  ].join(":");
})()
