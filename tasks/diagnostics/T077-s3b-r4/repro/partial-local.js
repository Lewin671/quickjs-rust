// Control: same function with the written name declared locally.
function f(n) {
    var a = 0, s;
    for (var k = 1; k <= n; k++) {
        s = k * 0.5;
        a += 1.0 / k;
    }
    return a;
}
var t = 0;
for (var j = 0; j < 20; j++) t += f(20000);
if (!(t > 0)) throw new Error("bad");
