(function () {
  var result = [];
  [new Map(), new Set()].forEach(function (p) {
    p.x = 1;
    var a = Object.create(p);
    result.push(Object.getPrototypeOf(a) === p, a.x);
    p.x = 2; result.push(a.x);
    var b = {}; Object.setPrototypeOf(b, p);
    result.push(Object.getPrototypeOf(b) === p, b.x);
    var c = {}; result.push(Reflect.setPrototypeOf(c, p), Reflect.getPrototypeOf(c) === p);
    p.y = 3; result.push(c.y);
  });
  return result.join('|');
})()
