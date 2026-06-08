(function () {
  var args = (function (a, b, c) {
    return arguments;
  })(1, 2, 3);
  args[Symbol.isConcatSpreadable] = true;
  var spread = [].concat(args, args);

  var holey = (function (a) {
    return arguments;
  })(1, 2, 3);
  delete holey[1];
  holey[Symbol.isConcatSpreadable] = true;
  var holeySpread = [].concat(holey, holey);

  var returned = (function (a) {
    arguments[0] = "x";
    return arguments;
  })("a");
  returned[0] = returned[0] + "y";

  return [
    spread.join("|"),
    holeySpread.join("|"),
    holeySpread.hasOwnProperty("1"),
    holeySpread.hasOwnProperty("4"),
    returned[0]
  ].join(":");
})()
