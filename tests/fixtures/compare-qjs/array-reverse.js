(function() {
  var xs = [1, 2, 3];
  var result = xs.reverse();
  var ys = [1, undefined, 3];
  ys.reverse();

  var object = { length: 4, 0: "a", 2: "c" };
  var objectResult = Array.prototype.reverse.call(object);

  var getterArray = ["first", "second"];
  Object.defineProperty(getterArray, "0", {
    get: function() {
      getterArray.length = 0;
      return "first";
    },
    configurable: true
  });
  getterArray.reverse();

  Array.prototype[1] = 1;
  var inheritedArray = [0];
  inheritedArray.length = 2;
  inheritedArray.reverse();
  var inherited = inheritedArray[0]
    + ":" + inheritedArray[1]
    + ":" + inheritedArray.hasOwnProperty("0")
    + ":" + inheritedArray.hasOwnProperty("1");
  delete Array.prototype[1];

  return (result === xs)
    + ":" + xs.join()
    + ":" + ys.length
    + ":" + ys.join()
    + ":" + (objectResult === object)
    + ":" + object[1]
    + ":" + object[3]
    + ":" + object.hasOwnProperty("0")
    + ":" + object.hasOwnProperty("2")
    + ":" + (Array.prototype.reverse.call(true) instanceof Boolean)
    + ":" + (0 in getterArray)
    + ":" + (1 in getterArray)
    + ":" + getterArray[1]
    + ":" + inherited;
})()
