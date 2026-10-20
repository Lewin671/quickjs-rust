(function () {
  function check(ok, label) { if (!ok) throw new Error(label); }
  function eq(a, b) { return JSON.stringify(a) === JSON.stringify(b); }
  check(eq('😀'.split('').map(function (s) { return s.charCodeAt(0); }), [55357, 56832]), 'empty separator');
  var texts = ['', 'abc', 'é中', '😀x😀', '\uD83Dx\uDE00', '\u{F0000}x\u{F07FF}'];
  var separators = ['', 'x', '😀', '\uD83D', '\uDE00', '\uDE00x\uD83D', '\uDB80', '\uDC00', 'missing'];
  for (var s of texts) for (var sep of separators) for (var limit of [0, 1, 2, 3, 99]) {
    var expected = [], start = 0;
    if (sep === '') {
      for (var i = 0; i < s.length && expected.length < limit; i++) expected.push(s.charAt(i));
    } else {
      for (var i = 0; i + sep.length <= s.length && expected.length < limit; i++) {
        if (s.slice(i, i + sep.length) === sep) {
          expected.push(s.slice(start, i)); start = i + sep.length; i = start - 1;
        }
      }
      if (expected.length < limit) expected.push(s.slice(start));
    }
    check(eq(s.split(sep, limit), expected), 'split units/limit');
  }
  var log = [];
  var receiver = { toString: function () { log.push('receiver'); return '😀'; } };
  var separator = { toString: function () { log.push('separator'); return ''; } };
  var limit = { valueOf: function () { log.push('limit'); return 1; } };
  check(String.prototype.split.call(receiver, separator, limit)[0] === '\uD83D', 'coerced split');
  check(log.join(',') === 'receiver,limit,separator', 'coercion order');
  return 'ok';
})()
