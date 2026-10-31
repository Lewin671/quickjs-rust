var keep = []; for (var i = 0; i < 2e5; i++) { var p = {a: i}; keep.push(Object.create(p)); } print('fan', keep.length);
