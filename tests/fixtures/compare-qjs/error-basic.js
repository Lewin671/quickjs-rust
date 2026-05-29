(function () {
  var constructed = new Error("boom");
  var called = Error("call");
  var thrown;
  try {
    throw new Error("thrown");
  } catch (error) {
    thrown = error.toString();
  }
  constructed.name = "Custom";
  return typeof Error + ":" +
    Error.length + ":" +
    (constructed instanceof Error) + ":" +
    constructed.message + ":" +
    constructed.toString() + ":" +
    called.toString() + ":" +
    new Error().toString() + ":" +
    Object.prototype.toString.call(new Error("tag")) + ":" +
    thrown + ":" +
    Object.getOwnPropertyDescriptor(constructed, "message").enumerable;
})()
