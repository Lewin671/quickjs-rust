(function () {
  var object = { first: 1, second: 2 };
  var proto = { inherited: 9 };
  var child = Object.create(proto, { own: { value: 3, enumerable: true } });
  return [
    Object.values.length,
    Object.values(object).join("|"),
    Object.values([4, 5]).join("|"),
    Object.values("ab").join("|"),
    Object.values(child).join("|"),
    Object.values(0).length
  ].join(":");
})()
