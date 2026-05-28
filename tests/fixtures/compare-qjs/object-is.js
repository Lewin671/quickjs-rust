(function () {
  var object = {};
  return [
    Object.is.length,
    Object.is(NaN, NaN),
    Object.is(+0, -0),
    Object.is(-0, -0),
    Object.is(1, 1),
    Object.is(1, "1"),
    Object.is(object, object),
    Object.is({}, {}),
    Object.is(),
    Object.is(0)
  ].join(":");
})()
