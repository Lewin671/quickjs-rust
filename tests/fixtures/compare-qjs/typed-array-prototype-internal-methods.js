(function () {
  var typedArray = new Uint8Array([7]);
  var array = [];
  var functionValue = function () {};
  Object.setPrototypeOf(array, typedArray);
  Object.setPrototypeOf(functionValue, typedArray);
  functionValue[1] = 8;
  return array[0] + ":" + ("0" in array) + ":"
    + functionValue.hasOwnProperty("1") + ":" + functionValue[1];
})()
