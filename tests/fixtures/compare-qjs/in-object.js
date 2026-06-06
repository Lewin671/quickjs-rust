(function () {
  var symbol = Symbol();
  var other = Symbol();
  var object = { answer: 42, [symbol]: 7 };
  return ("answer" in object) + ":" + (symbol in object) + ":" + (other in object);
})()
