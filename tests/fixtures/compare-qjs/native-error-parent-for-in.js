(function () {
  Error.enumProp = 1;
  var keys = "";
  for (var key in TypeError) keys += key;
  return keys;
})()
