(function () {
  var value = new String("abc");
  try { value.length = 1; } catch (error) {}
  return typeof value + ":" +
    (value.constructor === String) + ":" +
    value.valueOf() + ":" +
    value.toString() + ":" +
    value.length + ":" +
    value[1] + ":" +
    (value == "abc") + ":" +
    (value !== "abc") + ":" +
    Object.prototype.toString.call(value) + ":" +
    value.charAt(2);
})()
