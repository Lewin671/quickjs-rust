(function () {
  var seen = 0;
  var receiverOk = false;
  Object.defineProperty(Error, "x", {
    get: function () { return 3; },
    set: function (value) {
      seen = value;
      receiverOk = this === TypeError;
    },
    configurable: true
  });
  TypeError.x = 7;
  return seen + ":" + receiverOk + ":"
    + TypeError.hasOwnProperty("x") + ":" + TypeError.x;
})()
