(function () {
  var symbol = Symbol();
  var object = { red: 1, [symbol]: 2 };
  return (delete object.red) + ":" + (object.red === undefined) + ":" +
    (delete object[symbol]) + ":" + (object[symbol] === undefined);
})()
