(function () {
  var array = [1, 2];
  array[Symbol.isConcatSpreadable] = false;
  var kept = [0].concat(array);

  var arrayDefault = [1, 2];
  arrayDefault[Symbol.isConcatSpreadable] = undefined;
  var spread = [0].concat(arrayDefault);

  var object = { 0: "a", 2: "c", length: 3 };
  object[Symbol.isConcatSpreadable] = true;
  var objectSpread = [0].concat(object);

  var marker = {};
  var throwing = {};
  Object.defineProperty(throwing, Symbol.isConcatSpreadable, {
    get: function () {
      throw marker;
    }
  });
  var caught = false;
  try {
    [].concat(throwing);
  } catch (error) {
    caught = error === marker;
  }

  return [0].concat([1, 2], 3, [4]).join() + ":" +
    [].concat([0, 1], [2, 3]).length + ":" +
    [0].concat("x", true)[2] + ":" +
    kept.length + ":" + (kept[1] === array) + ":" +
    spread.join() + ":" +
    objectSpread.length + ":" + objectSpread[1] + ":" +
    objectSpread.hasOwnProperty("2") + ":" + objectSpread[3] + ":" +
    caught;
})()
