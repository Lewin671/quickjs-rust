(function () {
  var custom = {};
  custom[Symbol.toStringTag] = "custom";
  var fallback = {};
  fallback[Symbol.toStringTag] = 86;
  return ({}).toString() + ":" + Object.prototype.toString.length + ":" +
    Object.prototype.toString.call(custom) + ":" +
    Object.prototype.toString.call(fallback);
})()
