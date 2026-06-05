(function () {
  return Math.f16round.length + ":" +
    Math.f16round(1.00048828125) + ":" +
    Math.f16round(1.0009765625) + ":" +
    Math.f16round(65519) + ":" +
    (Math.f16round(65520) === Infinity) + ":" +
    (1 / Math.f16round(-0) === -Infinity) + ":" +
    (Math.f16round(NaN) !== Math.f16round(NaN));
})()
