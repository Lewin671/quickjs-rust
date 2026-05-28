// Derived from: test/built-ins/Function/prototype/bind/S15.3.4.5_A5.js
function Point(x, y) { this.x = x; this.y = y; }
var Bound = Point.bind({ ignored: true }, 2);
var point = new Bound(3);
if (point.x !== 2 || point.y !== 3) {
  throw "constructing a bound function should use bound arguments";
}
if (!(point instanceof Point)) {
  throw "bound function construction should use the target prototype";
}
