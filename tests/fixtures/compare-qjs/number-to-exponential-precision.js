(function () {
  return Number.prototype.toExponential.length + ":" +
    Number.prototype.toPrecision.length + ":" +
    (12.345).toExponential() + ":" +
    (12.345).toExponential(2) + ":" +
    (1).toExponential(0) + ":" +
    (25).toExponential(0) + ":" +
    (12345).toExponential(3) + ":" +
    (-0).toExponential(2) + ":" +
    NaN.toExponential(101) + ":" +
    Infinity.toExponential(101) + ":" +
    (123.456).toPrecision() + ":" +
    (123.456).toPrecision(5) + ":" +
    (123.456).toPrecision(2) + ":" +
    (0.0001234).toPrecision(5) + ":" +
    (1e-7).toPrecision(2) + ":" +
    (new Number(7)).toPrecision(3);
})()
