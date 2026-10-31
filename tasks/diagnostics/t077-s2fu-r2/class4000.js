var C = class {}; for (var i = 0; i < 4000; i++) C = class extends C {};
if (typeof C !== 'function') throw new Error('class chain');
print('class4000'); C = null;
