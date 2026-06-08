(function () {
  var originalAdd = Set.prototype.add;
  var calls = 0;
  var receivers = [];
  var seen = "";
  Set.prototype.add = function (value) {
    calls = calls + 1;
    receivers.push(this);
    seen = seen + value + "|";
    return originalAdd.call(this, value);
  };
  var set = new Set(["a", "b"]);

  var nullAdderNoIterableSize = 0;
  var nullAdderThrows = false;
  Set.prototype.add = null;
  try {
    nullAdderNoIterableSize = new Set().size;
  } catch (error) {
    nullAdderNoIterableSize = -1;
  }
  try {
    new Set([1]);
  } catch (error) {
    nullAdderThrows = error.constructor === TypeError;
  }

  return calls + ":" + seen + ":" + (receivers[0] === set) + ":" +
    (receivers[1] === set) + ":" + set.has("b") + ":" +
    nullAdderNoIterableSize + ":" + nullAdderThrows;
})()
