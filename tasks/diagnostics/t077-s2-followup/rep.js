var ps = []; for (var i = 0; i < 60000; i++) { var p = {a: i}; Object.create(p); ps.push(p); }
// keep children alive so the users stay registered
var kids = ps.map(function (p) { return Object.create(p); });
var Q = {q: 1}; var t0 = Date.now();
for (var i = 0; i < 60000; i++) Object.setPrototypeOf(ps[i], Q);
print('reparent', 60000, Date.now() - t0);
