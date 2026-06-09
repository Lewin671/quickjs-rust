(function () {
  var buffer = new ArrayBuffer(8);
  var rejected = false;
  try {
    buffer.constructor = true;
    buffer.slice();
  } catch (error) {
    rejected = error instanceof TypeError;
  }
  buffer.constructor = undefined;
  return typeof ArrayBuffer + ":" + ArrayBuffer.length + ":" +
    buffer.byteLength + ":" + buffer.slice(2, 6).byteLength + ":" +
    buffer.slice(-5, -1).byteLength + ":" +
    (Object.getPrototypeOf(buffer.slice()) === ArrayBuffer.prototype) + ":" +
    Object.prototype.toString.call(buffer) + ":" + rejected;
})()
