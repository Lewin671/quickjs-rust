(function () {
  var C = '\u{1F600}', hi = '\uD83D', lo = '\uDE00';
  function check(ok, label) { if (!ok) throw new Error(label); }
  var producers = [
    ['plus', function () { return hi + lo; }],
    ['compound', function () { var s = hi; s += lo; return s; }],
    ['local-literal', function () { var s = hi; s += '\uDE00'; return s; }],
    ['coercion', function () { return hi + { toString: function () { return lo; } }; }],
    ['template', function () { return `${hi}${lo}`; }],
    ['raw', function () { return String.raw({raw: [hi, lo]}); }],
    ['raw-substitution', function () { return String.raw({raw: [hi, '']}, lo); }],
    ['concat', function () { return hi.concat(lo); }],
    ['char-code', function () { return String.fromCharCode(0xD83D, 0xDE00); }],
    ['code-point', function () { return String.fromCodePoint(0xD83D, 0xDE00); }],
    ['join', function () { return [hi, lo].join(''); }],
    ['join-separator', function () { return [hi, ''].join(lo); }],
    ['repeat', function () { return (lo + hi).repeat(2).slice(1, 3); }],
    ['pad-start', function () { return lo.padStart(2, hi); }],
    ['pad-end', function () { return hi.padEnd(2, lo); }],
    ['pad-cycle', function () { return ''.padStart(4, lo + hi).slice(1, 3); }],
    ['json', function () { return JSON.parse('"\\uD83D\\uDE00"'); }],
    ['lower', function () { return C.toLowerCase(); }],
    ['upper', function () { return C.toUpperCase(); }],
    ['normalize', function () { return (hi + lo).normalize(); }],
    ['well-formed', function () { return (hi + lo).toWellFormed(); }],
    ['replace', function () { return (hi + 'x').replace('x', lo); }],
    ['replace-all', function () { return (hi + 'x').replaceAll('x', lo); }],
    ['replace-callback', function () { return (hi + 'x').replace('x', function () { return lo; }); }],
    ['regexp-captures', function () { return C.replace(/(.)(.)/, '$1$2'); }],
    ['regexp-suffix', function () { return (hi + 'x').replace(/x/, lo); }],
    ['regexp-substitution', function () { return (hi + 'x').replace(/x/, "$`" + lo).slice(1); }],
    ['unescape', function () { return unescape('%uD83D%uDE00'); }]
  ];
  function factories() {
    return [
      function () { return {}; },
      function () { return { '\u{1F600}': 0, a: 0 }; },
      function () { return { '\u{1F600}': 0, a: 0, b: 0 }; },
      function () { var o = {}; for (var i = 0; i < 14; i++) o['p' + i] = i; return o; },
      function () { return []; },
      function () { return function () {}; },
      function () { return class { static ['\u{1F600}'] = 0; }; },
      function () { return Object.fromEntries([[C, 0]]); },
      function () { return JSON.parse('{"\\uD83D\\uDE00":0}'); }
    ];
  }
  for (var p of producers) {
    var built = p[1]();
    check(built === C && built.length === 2, p[0] + ' value');
    for (var factory of factories()) {
      for (var reverse of [false, true]) {
        var o = factory(), stored = reverse ? built : C, queried = reverse ? C : built;
        var before = Object.keys(o).filter(function (k) { return k !== C; }).length;
        o[stored] = 1;
        check(o[queried] === 1 && queried in o && Object.prototype.hasOwnProperty.call(o, queried), p[0] + ' get');
        check(Object.getOwnPropertyDescriptor(o, queried).value === 1, p[0] + ' descriptor');
        o[queried] = 2;
        check(o[stored] === 2 && Object.keys(o).length === before + 1, p[0] + ' one key');
        check(JSON.stringify({[queried]: o[stored]}) === '{"' + C + '":2}', p[0] + ' key serialization');
        if (typeof o === 'object' && !Array.isArray(o)) {
          var json = JSON.stringify(o);
          check(json.split('"' + C + '":').length === 2, p[0] + ' stringify');
        }
      }
    }
    check(Symbol.for(C) === Symbol.for(built), p[0] + ' symbol');
    check(C.indexOf(built) === 0 && built.indexOf(C) === 0, p[0] + ' indexOf');
    check(C.lastIndexOf(built) === 0 && built.lastIndexOf(C) === 0, p[0] + ' lastIndexOf');
    check(C.includes(built) && built.includes(C), p[0] + ' includes');
    check(C.startsWith(built) && built.startsWith(C) && C.endsWith(built) && built.endsWith(C), p[0] + ' ends');
    check(('a' + C + 'b').split(built).join(':') === 'a:b', p[0] + ' split');
    check(C.replace(built, 'ok') === 'ok' && built.replace(C, 'ok') === 'ok', p[0] + ' string replace');
    check(C.replaceAll(built, 'ok') === 'ok' && built.replaceAll(C, 'ok') === 'ok', p[0] + ' string replaceAll');
    check(C.localeCompare(built) === 0 && built.localeCompare(C) === 0, p[0] + ' localeCompare');
    var target = {};
    Object.defineProperty(target, C, {value: 1, configurable: false});
    var proxy = new Proxy(target, {ownKeys: function () { return [built]; }});
    check(Reflect.ownKeys(proxy).length === 1, p[0] + ' proxy');
  }
  check(JSON.stringify({a:{b:0}}, null, lo + hi).includes(C), 'JSON indentation seam');
  var name = '\uD801' + '\uDC00';
  globalThis[name] = 42;
  check((0, eval)('\u{10400}') === 42, 'global identifier');
  var scope = {}; scope[name] = 43;
  with (scope) { check(eval('\u{10400}') === 43, 'with identifier'); }
  delete globalThis[name];
  var globalAppend = hi; globalAppend += '\uDE00';
  check(globalAppend === C && ({[globalAppend]: 1})[C] === 1, 'global literal append');
  var holder = {s: hi}; holder.s += lo;
  check(({[holder.s]: 1})[C] === 1, 'property append');
  var escaped = '\u{F0000}';
  var rebuilt = String.fromCharCode(0xDB80, 0xDC00);
  check(escaped === rebuilt && escaped.length === 2 && escaped.charCodeAt(0) === 0xDB80, 'sentinel units');
  check(JSON.parse(JSON.stringify(escaped)) === escaped && JSON.stringify({[escaped]: 1}) === '{"' + escaped + '":1}', 'sentinel JSON');
  check(escaped !== '\uD800' && Symbol.for(escaped) !== Symbol.for('\uD800'), 'sentinel distinct');
  return 'canonical strings: ' + producers.length + ' producers';
})()
