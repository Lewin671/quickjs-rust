(function(){
  var errors=[];function check(n,a,e){if(a!==e)errors.push(n+':'+a+'!='+e);}
  function f(a,a){return a;}function g(a,b,a){return [a,b].join('/');}
  function h(x){return String(f(x))+'|'+f(x,9)+'|'+g(1,2)+'|'+g(1,2,3);}
  function k(a,a){var t=a;return t===undefined?'U':t;}
  function caller(n){return k(n)+k(n,n+1);}
  function m(a,b,a){return a+1;}
  var o={m:m};for(var i=0;i<100;i++){check('h',h(i),'undefined|9|/2|3/2');check('caller',caller(i),'U'+(i+1));check('method',String(o.m(i))+'/'+o.m(i,1,2),'NaN/3');}
  check('missing',String(f(1)),'undefined');check('last',f(1,2),2);check('map',[1,2].map(f).join(','),'0,1');
  function mapped(a,a){a=7;var before=arguments[0]+':'+arguments[1];arguments[0]=8;var zero=a;arguments[1]=9;return before+':'+zero+':'+a;}
  check('mapped short',mapped(1),'1:undefined:7:7');check('mapped full',mapped(1,2),'1:7:7:9');
  function erased(a,a){delete arguments[1];a=6;return arguments[0]+':'+arguments[1]+':'+a;}
  check('delete mapping',erased(1,2),'1:undefined:6');
  function captured(a,b,a){return [function(){return a;},function(v){a=v;}];}
  var x=captured(1,2),y=captured(1,2,3);check('capture missing',String(x[0]()),'undefined');check('capture last',y[0](),3);x[1](5);check('capture write',x[0](),5);check('capture independent',y[0](),3);
  function directEval(a,a){return eval('arguments[1]')+':'+a;}
  check('eval arguments',directEval(1,2),'2:2');
  var invalid=['function f(a,a){"use strict";}','(a,a)=>a','function f(a,a=1){}','function f(a,{a}){}','function f(a,...a){}','({m(a,a){}})'];
  for(var j=0;j<invalid.length;j++){var rejected=false;try{Function(invalid[j]);}catch(e){rejected=e instanceof SyntaxError;}check('syntax'+j,rejected,true);}
  return errors.length?errors.join('|'):'ok';
})()
