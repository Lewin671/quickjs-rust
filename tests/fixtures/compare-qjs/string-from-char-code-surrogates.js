(function () {
  var value = String.fromCharCode(0xd800, 0xdc00);
  return value.length + ":" +
    value.charCodeAt(0) + ":" +
    value.charCodeAt(1) + ":" +
    (value === "\ud800\udc00");
})()
