(function () {
  var xs = [1, 2, 3, 4];
  var removed = xs.splice(1, 2, "a", "b", "c");
  var ys = [1, 2, 3];
  var tail = ys.splice(-2);
  var zs = [1, undefined, 3];
  var undef = zs.splice(1, 1, 2);
  var obj = {0: 0, 1: 1, 2: 2, 3: 3, length: 4};
  var generic = Array.prototype.splice.call(obj, 0, 3, 4, 5);
  var rangeError = false;
  var big = Object.defineProperty({}, "length", {
    get: function () {
      return Math.pow(2, 32);
    },
    set: function () {
      throw "length should not be set";
    }
  });
  try {
    Array.prototype.splice.call(big, 0);
  } catch (error) {
    rangeError = error instanceof RangeError;
  }
  var constructorTypeError = false;
  var invalidConstructor = [];
  invalidConstructor.constructor = 1;
  try {
    invalidConstructor.splice();
  } catch (error) {
    constructorTypeError = error instanceof TypeError;
  }
  var marker = {ok: true};
  var constructorAbrupt = false;
  var poisonedConstructor = [];
  Object.defineProperty(poisonedConstructor, "constructor", {
    get: function () {
      throw marker;
    }
  });
  try {
    poisonedConstructor.splice();
  } catch (error) {
    constructorAbrupt = error === marker;
  }
  return removed.join()
    + ":" + xs.join()
    + ":" + tail.join()
    + ":" + ys.join()
    + ":" + (undef[0] === undefined)
    + ":" + zs.join()
    + ":" + generic.join()
    + ":" + obj.length
    + ":" + obj[0]
    + ":" + obj[1]
    + ":" + obj[2]
    + ":" + obj[3]
    + ":" + Array.prototype.splice.call(true).length
    + ":" + rangeError
    + ":" + constructorTypeError
    + ":" + constructorAbrupt
    + ":" + Array.prototype.splice.length;
})()
