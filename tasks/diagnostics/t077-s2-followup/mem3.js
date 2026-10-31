var P = {x: 1}; var s = 0;
function rd(o) { return o.x; }
for (var i = 0; i < 1e6; i++) { var c = Object.create(P); var d = Object.create(c); s += rd(d); }
print('churn', s);
