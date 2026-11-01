// Function-local loop that writes an implicit global (configurable property).
function f(n) {
    var a = 0;
    for (var k = 1; k <= n; k++) {
        s = k * 0.5;      // implicit global write
        a += 1.0 / k;
    }
    return a;
}
var t = 0;
for (var j = 0; j < 20; j++) t += f(20000);
if (!(t > 0)) throw new Error("bad");
