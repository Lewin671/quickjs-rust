(function() {
  var key = "answer";
  var symbol = Symbol("id");
  var object = { [key]: 40 + 2, [1 + 1]: "two", [symbol]: 7 };
  object[symbol] = object[symbol] + 1;

  var target = { [symbol]: 1 };
  Object.seal(target);
  Object.assign(target, { [symbol]: 3 });

  return object.answer + ":" + object[2] + ":" + object[symbol] + ":" + target[symbol];
})()
