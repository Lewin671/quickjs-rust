(function() { var xs = [1, 2, 3]; var result = xs.reverse(); var ys = [1, undefined, 3]; ys.reverse(); return (result === xs) + ":" + xs.join() + ":" + ys.length + ":" + ys.join(); })()
