(function () {
  function add(a, b, c) { return this.base + a + b + c; }
  function Point(x, y) { this.x = x; this.y = y; }
  let context = { base: 4 };
  let bound = add.bind(context, 2);
  let rebound = bound.bind({ ignored: true }, 3);
  let BoundPoint = Point.bind({ ignored: true }, 5);
  let point = new BoundPoint(6);
  return bound(3, 4) + ":" + rebound(4) + ":" + bound.length + ":" + Function.prototype.bind.length + ":" + point.x + ":" + point.y + ":" + (point instanceof Point) + ":" + Object.hasOwn(BoundPoint, "prototype");
})()
