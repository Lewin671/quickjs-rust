var keep = []; for (var i = 0; i < 2e5; i++) { var p = {a: i, m() { return 1; }}; var c = Object.create(p); c.m(); keep.push(c); } print('fan-call', keep.length);
