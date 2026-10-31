var p = {};
for (var i = 0; i < 1000000; i++) { p = Object.create(p); p.v = i; }
if (p.v !== 999999) throw new Error('owned chain');
print('owned-chain', p.v);
