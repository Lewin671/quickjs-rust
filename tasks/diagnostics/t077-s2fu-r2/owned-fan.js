var keep = [];
for (var i = 0; i < 250000; i++) {
  var p = {}; var c = Object.create(p); c.a = i; c.b = i + 1; keep.push(c);
}
if (keep.length !== 250000 || keep[249999].b !== 250000) throw new Error('owned fan');
print('owned-fan', keep.length);
