var keep = []; for (var i = 0; i < 1e5; i++) { var p = {a: i, b: i, c: i, d: i, e: i, f: i, g: i, h: i, j: i, k: i}; keep.push(Object.create(p)); } print('fan10', keep.length);
