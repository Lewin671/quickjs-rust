(function () {
  var errors=[];
  function check(name,actual,expected){if(actual!==expected)errors.push(name+':'+actual+'!='+expected);}
  function f(kind){var log='';function g(){outer:for(var i=0;i<2;i++){try{try{return 1;}finally{try{throw 'local';}catch(e){log+='x';}try{log+='y';}finally{log+='z';}if(kind===0)break outer;if(kind===1)return 2;if(kind===2)throw 3;}}finally{log+='o';}}return 4;}try{return g()+':'+log;}catch(e){return e+':'+log;}}
  check('break',f(0),'4:xyzo');check('return',f(1),'2:xyzo');check('throw',f(2),'3:xyzo');
  function innerBreak(){try{return 5;}finally{for(var i=0;i<2;i++){try{break;}finally{}}}}
  check('contained break',innerBreak(),5);
  function pendingThrow(){try{throw 8;}finally{try{throw 9;}catch(e){}try{}finally{}}}
  try{pendingThrow();}catch(e){check('preserve throw',e,8);}
  function overrideJump(){var log='';for(var i=0;i<2;i++){try{try{continue;}finally{break;}}finally{log+='o';}}return log;}
  check('override continue',overrideJump(),'o');
  return errors.length?errors.join('|'):'ok';
})()
