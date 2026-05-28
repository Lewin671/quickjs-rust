(function() { var xs = [1, 2, 3, 4]; var result = xs.fill(0, -3, -1); var ys = [1, 2, 3]; ys.fill(); return (result === xs) + ":" + xs.join() + ":" + ys.length + ":" + ys.join(); })()
