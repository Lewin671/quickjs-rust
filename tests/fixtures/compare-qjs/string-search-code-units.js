(function () {
  function check(ok, label) { if (!ok) throw new Error(label); }
  function sameAt(s, n, p) {
    if (p < 0 || p + n.length > s.length) return false;
    for (var j = 0; j < n.length; j++) if (s.charCodeAt(p + j) !== n.charCodeAt(j)) return false;
    return true;
  }
  var texts = ['', 'ascii', 'é中', '😀x😀', '\uD83Dx\uDE00', '\u{F0000}x\u{F07FF}'];
  var needles = ['', 'a', 'x', 'é', '中', '😀', '\uD83D', '\uDE00', '\uDE00x\uD83D', '\uDB80', '\uDC00', '\u{F0000}'];
  var positions = [-Infinity, -2, -0.5, 0, 1, 2, 3, 99, Infinity, NaN, undefined];
  for (var s of texts) for (var n of needles) for (var pos of positions) {
    var p = Number.isNaN(pos) || pos === undefined ? 0 : Math.max(0, Math.min(s.length, Math.trunc(pos)));
    var last = Number.isNaN(pos) || pos === undefined ? s.length : p;
    var end = pos === undefined ? s.length : p;
    var first = -1, reverse = -1;
    for (var i = p; i <= s.length; i++) if (sameAt(s, n, i)) { first = i; break; }
    for (var i = last; i >= 0; i--) if (sameAt(s, n, i)) { reverse = i; break; }
    check(s.indexOf(n, pos) === first, 'indexOf');
    check(s.includes(n, pos) === (first !== -1), 'includes');
    check(s.lastIndexOf(n, pos) === reverse, 'lastIndexOf');
    check(s.startsWith(n, pos) === sameAt(s, n, p), 'startsWith');
    check(s.endsWith(n, pos) === sameAt(s, n, end - n.length), 'endsWith');
  }
  check('😀x'.indexOf('x') === 2, 'astral index');
  check('😀x'.startsWith('x', 2), 'astral start');
  check('😀'.includes('\uDE00'), 'half pair');
  return 'ok';
})()
