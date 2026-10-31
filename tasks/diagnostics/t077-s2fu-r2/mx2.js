// Reconstructed pending exact lead source: half-million one-field chain.
var p = {};
for (var i = 0; i < 500000; i++) { p = Object.create(p); p.v = i; }
if (p.v !== 499999) throw new Error('mx2');
print('mx2', p.v);
