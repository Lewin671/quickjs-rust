var C = class {}; for (var i = 0; i < 2000; i++) { C = class extends C {}; } print('built'); C = null; print('dropped');
