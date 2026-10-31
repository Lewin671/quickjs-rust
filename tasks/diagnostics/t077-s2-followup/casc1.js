var keep = [];
for (var i = 0; i < 1e5; i++) { var p = {a: i}; keep.push(Object.create(p)); }
var t0 = Date.now();
for (var j = 0; j < 200; j++) { Object.prototype['pk' + j] = j; }
for (var j = 0; j < 200; j++) { delete Object.prototype['pk' + j]; }
print('cascade ms', Date.now() - t0);
