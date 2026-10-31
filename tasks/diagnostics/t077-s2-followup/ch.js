var a = {}; for (var i = 0; i < 6000; i++) { var b = {}; Object.setPrototypeOf(b, a); a = b; } print('x');
