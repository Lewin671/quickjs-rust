(function () {
  var errors=[];
  function check(n,a,e){if(a!==e)errors.push(n+':'+a+'!='+e);}
  function run(kind){var log='';function iter(name){return {[Symbol.iterator]:function(){return {next:function(){return {value:1,done:false};},return:function(){log+=name;return {};}};}};}function f(){outer:for(var i=0;i<2;i++){for(var x of iter('X')){try{for(var y of iter('Y')){try{try{if(kind===0)break outer;if(kind===1)continue outer;if(kind===2)return 7;throw 8;}finally{log+='a';}}finally{log+='b';}}}finally{log+='c';}}}return 9;}try{return f()+':'+log;}catch(e){return e+':'+log;}}
  for(var hot=0;hot<40;hot++){check('break',run(0),'9:abYcX');check('continue',run(1),'9:abYcXabYcX');check('return',run(2),'7:abYcX');check('throw',run(3),'8:abYcX');}
  function cancel(){var log='';var iterable={[Symbol.iterator]:function(){return {next:function(){return {value:1,done:false};},return:function(){log+='close';return {};}};}};for(var x of iterable){try{break;}finally{log+='f';if(log==='f')continue;}}return log;}
  check('cancel break',cancel(),'ffclose');
  return errors.length?errors.join('|'):'ok';
})()
