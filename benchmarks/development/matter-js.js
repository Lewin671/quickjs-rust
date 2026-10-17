// Development driver: matter-js (MIT) simulates a generated body stack.
// Deterministic: fixed bodies, fixed delta, no clock; prints one line.
var BODIES = 40;
var STEPS = 900;

var engine = Matter.Engine.create({ enableSleeping: false, gravity: { x: 0, y: 0.5, scale: 0.001 } });
var world = engine.world;

var ground = Matter.Bodies.rectangle(400, 700, 900, 40, { isStatic: true });
Matter.Composite.add(world, ground);
Matter.Composite.add(world, Matter.Bodies.circle(80, 100, 12, { restitution: 0.4 }));

for (var i = 0; i < BODIES; i++) {
  var x = 120 + (i % 8) * 70;
  var y = 80 + Math.floor(i / 8) * 60;
  var body = i % 3 === 0
    ? Matter.Bodies.rectangle(x, y, 34, 20, { friction: 0.2 })
    : Matter.Bodies.circle(x, y, 11, { restitution: 0.3 });
  Matter.Composite.add(world, body);
}

var hash = 0;
function mix(value) {
  hash = ((hash << 5) - hash + value) | 0;
}

for (var step = 0; step < STEPS; step++) {
  Matter.Engine.update(engine, 16.666);
}

var bodies = Matter.Composite.allBodies(world);
mix(bodies.length);
for (var i = 0; i < bodies.length; i++) {
  var body = bodies[i];
  mix(Math.round(body.position.x * 1000) % 100000);
  mix(Math.round(body.position.y * 1000) % 100000);
  mix(Math.round(body.angle * 10000) % 10000);
  mix(Math.round(body.speed * 1000));
}
mix(engine.pairs.list.length);
mix(Math.round(engine.timing.timestamp) % 1000000);
mix(Matter.Vector.magnitude({ x: 3, y: 4 }) === 5 ? 1 : 0);

var query = Matter.Query.region(bodies, Matter.Bounds.create([{ x: 0, y: 0 }, { x: 800, y: 800 }]));
mix(query.length);
var bounds = Matter.Composite.bounds(world);
mix(Math.round(bounds.min.x) + Math.round(bounds.max.y));
print("CHECKSUM " + (hash >>> 0).toString(16));
