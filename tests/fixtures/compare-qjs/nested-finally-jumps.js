(function () {
  var errors = [];
  function check(name, actual, expected) { if (actual !== expected) errors.push(name + ':' + actual + '!=' + expected); }
  var loops = ['for(var i=0;i<2;i++)', 'for(var i in {x:1,y:1})', 'for(var i of [0,1])', 'while(i++<2)', 'for(var i=0;i<2;i++) switch(i) { default: BODY }'];
  for (var depth = 2; depth <= 3; depth++) {
    var one = depth === 2 ? 'ab' : 'abc';
    for (var kind = 0; kind < 4; kind++) {
      for (var form = 0; form < loops.length; form++) {
        var jump = ['break;', 'continue;', 'break outer;', 'continue outer;'][kind];
        var body = jump;
        for (var j = 0; j < depth; j++) body = 'try {' + body + '} finally { log += "' + 'abc'[j] + '"; }';
        var loop = loops[form];
        loop = loop.indexOf('BODY') >= 0 ? loop.replace('BODY', body) : loop + '{' + body + '}';
        var fn = Function('var log="",i=0; outer:' + loop + ';return log;');
        var expected = kind % 2 === 1 || (form === 4 && kind === 0) ? one + one : one;
        for (var hot = 0; hot < 40; hot++) check(depth + '/' + kind + '/' + form, fn(), expected);
      }
    }
  }
  function retained() { var log=''; try { for(var i=0;i<2;i++){try {try {continue;}finally{log+='a';}}finally{log+='b';}} log+='c'; } finally {log+='d';} return log; }
  check('retained', retained(), 'ababcd');
  function returned() { var log=[]; function f(){try {try {try {return 7;}finally{log.push('a');}}finally{log.push('b');}}finally{log.push('c');}} return f()+':'+log.join(''); }
  check('return', returned(), '7:abc');
  return errors.length ? errors.join('|') : 'ok';
})()
