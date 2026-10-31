// prototype objects used as data holders: many writes
var P = {x: 0, y: 0}; var o = Object.create(P); var s = 0;
for (var i = 0; i < 2e6; i++) { P.x = i; s += o.x; }
print('holder', s);
