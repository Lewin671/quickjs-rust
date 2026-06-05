(function () {
  var desc = Object.getOwnPropertyDescriptor(RegExp.prototype, "dotAll");
  var get = desc.get;
  var rejected = false;
  try {
    get.call({});
  } catch (error) {
    rejected = error.constructor === TypeError;
  }
  return /a/s.dotAll + ":" +
    /a/.dotAll + ":" +
    RegExp.prototype.dotAll + ":" +
    (typeof get) + ":" +
    desc.set + ":" +
    desc.enumerable + ":" +
    desc.configurable + ":" +
    rejected;
})()
