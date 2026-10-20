(function () {
  function check(ok, label) { if (!ok) throw new Error(label); }
  var s = '😀é😀é😀';
  check(s.indexOf('😀', 1) === 3, 'skip interior pair');
  check(s.lastIndexOf('é😀', 4) === 2, 'reverse end bound');
  check(s.split('é😀').map(x => x.length).join(',') === '2,0,0', 'byte split');
  check(s.replaceAll('é😀', (m, p) => '[' + p + ']') === '😀[2][5]', 'byte callbacks');
  check(s.replace('é😀', (m, p) => '[' + p + ']') === '😀[2]é😀', 'first callback');
  check(s.replaceAll('\uDE00', (m, p) => p).length === 8, 'half-pair callbacks');
  for (var scalar of ['\u{F0000}', '\u{F07FF}']) {
    var t = '😀' + scalar + 'é' + scalar;
    check(t.indexOf(scalar, 1) === 2, 'escaped scalar first');
    check(t.lastIndexOf(scalar) === 5, 'escaped scalar last');
    check(t.split(scalar).map(x => x.length).join(',') === '2,1,0', 'escaped scalar split');
    check(t.replaceAll(scalar, (m, p) => p) === '😀2é5', 'escaped scalar callbacks');
  }
  for (var prefix of ['a', '\uD800']) {
    var text = 'é' + prefix.repeat(20000);
    var needle = prefix.repeat(1000) + 'b';
    check(text.indexOf(needle) === -1, 'prefix forward');
    check(text.lastIndexOf(needle) === -1, 'prefix reverse');
    check(!text.includes(needle), 'prefix includes');
    check(text.split(needle).length === 1, 'prefix split');
    check(text.replace(needle, '!') === text, 'prefix replace');
    check(text.replaceAll(needle, '!') === text, 'prefix replaceAll');
  }
  return 'ok';
})()
