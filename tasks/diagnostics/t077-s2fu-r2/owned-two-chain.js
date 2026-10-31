// Additional two-field chain control.
var p = {};
for (var i = 0; i < 500000; i++) { p = Object.create(p); p.v = i; p.w = i + 1; }
if (p.v !== 499999 || p.w !== 500000) throw new Error('owned two-field chain');
print('owned-two-chain', p.v, p.w);
