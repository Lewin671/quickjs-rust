(function () {
function f(n) {
    var a = Array(n), b = Array(n), k = 0, s = 0, t = "";
    for (var i = 0; i < n; i++) a[i] = i;
    while (true) {
        if (k < 30) { t = ""; for (var i = 0; i < n; i++) t += (a[i] + 1).toString(); }
        k++;
        for (var i = 0; i < n; i++) b[i] = a[i];
        for (var j = 0; j < n; j++) s += b[j] & 7;
        if (k == 1000) return s;
    }
}
return f(8);
})()
