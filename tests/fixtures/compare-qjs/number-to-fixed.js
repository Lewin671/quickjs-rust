(function () {
  return Number.prototype.toFixed.length + ":" +
    Number.prototype.toFixed.call(0) + ":" +
    (3).toFixed(2) + ":" +
    (1.25).toFixed(1) + ":" +
    (123.456).toFixed(1) + ":" +
    (1000000000000000128).toFixed(0) + ":" +
    (-0).toFixed(2) + ":" +
    (1e21).toFixed(2) + ":" +
    NaN.toFixed(3) + ":" +
    (new Number(7)).toFixed(2) + ":" +
    (3).toFixed("2.9");
})()
