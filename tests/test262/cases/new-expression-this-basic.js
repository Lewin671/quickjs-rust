// Derived from: test/language/expressions/new/S11.2.2_A1.1.js
function Point(x, y) {
  this.x = x;
  this.y = y;
}

var point = new Point(2, 3);
if (point.x + point.y !== 5) { throw; }
