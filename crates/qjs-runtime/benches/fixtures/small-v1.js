// Rust-native lifecycle fixture v1: small.
function makeCounter(start) {
  let value = start;
  return function step(delta) {
    value = value + delta;
    return value;
  };
}

const counter = makeCounter(3);
const point = { x: 2, y: 5, move(dx, dy) { this.x += dx; this.y += dy; } };
const values = [1, 2, 3, 4, 5, 6];
let total = 0;

for (let index = 0; index < values.length; index += 1) {
  if (index % 2 === 0) {
    total += counter(values[index]);
  } else {
    point.move(values[index], index);
    total += point.x + point.y;
  }
}

total;
