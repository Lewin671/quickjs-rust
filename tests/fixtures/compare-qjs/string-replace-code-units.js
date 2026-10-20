(function () {
  function check(ok, label) { if (!ok) throw new Error(label); }
  check('😀x'.replace('x', function (m, p) { return p; }) === '😀2', 'callback position');
  check('😀'.replace('\uDE00', 'x') === '\uD83Dx', 'low half');
  check('😀'.replace('\uD83D', 'x') === 'x\uDE00', 'high half');
  check('😀x😀'.replaceAll('\uDE00', '|') === '\uD83D|x\uD83D|', 'all halves');
  check('😀'.replaceAll('', '|') === '|\uD83D|\uDE00|', 'all empty');
  check('😀'.replaceAll('', function (m, p) { return p; }) === '0\uD83D1\uDE002', 'empty callback positions');
  check('😀'.replace('\uDE00', "$`:$&:$'") === '\uD83D\uD83D:\uDE00:', 'low substitutions');
  check('😀'.replace('\uD83D', "$`:$&:$'") === ':\uD83D:\uDE00\uDE00', 'high substitutions');
  check('😀x'.replaceAll('x', '$`') === '😀😀', 'astral prefix');
  check('😀x'.replaceAll('😀', "$'") === 'xx', 'astral suffix');
  check('\u{F0000}'.replace('\uDB80', 'x') === 'x\uDC00', 'sentinel scalar high');
  check('\u{F0000}'.replaceAll('', '|') === '|\uDB80|\uDC00|', 'sentinel scalar empty');
  check('😀x'.search('x') === 2 && '😀'.search('\uDE00') === 1, 'string regexp search');
  check('😀x'.match('x').index === 2 && '😀'.match('\uDE00')[0] === '\uDE00', 'string regexp match');
  check('😀'.at(1) === '\uDE00' && '😀'.at(-2) === '\uD83D', 'at units');
  check(JSON.stringify(Array.from('😀\uD83Dx\uDE00').map(function (s) { return s.length; })) === '[2,1,1,1]', 'iterator code points');
  return 'ok';
})()
