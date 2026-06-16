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
    value.charAt(2) + ":" +
    (String.prototype == "") + ":" +
    String.prototype.valueOf() + ":" +
    String.prototype.length + ":" +
    Object.prototype.isPrototypeOf(String.prototype) + ":" +
    (delete String.prototype.toString,
      Object.prototype.toString.call(String.prototype));
})()
