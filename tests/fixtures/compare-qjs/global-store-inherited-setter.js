(function(){var log=[];
Object.defineProperty(Object.prototype,'gsAcc',{get(){return 10;},set(v){log.push('set:'+v);},configurable:true});
Object.defineProperty(Object.prototype,'gsRo',{value:5,writable:false,configurable:true});
Object.prototype.gsCnt=0;
function own(n){return Object.prototype.hasOwnProperty.call(globalThis,n)?'own='+globalThis[n]:'none';}
function mk(n){Object.defineProperty(globalThis,n,{value:1,writable:true,configurable:true});}
function a(){gsAcc+=1;gsAcc++;return own('gsAcc');}
function b(){'use strict';gsAcc||=1;gsAcc+=1;return own('gsAcc');}
function c(){gsRo+=1;return own('gsRo');}
function d(){'use strict';gsRo+=1;}
function e(){'use strict';gsCnt++;return globalThis.gsCnt;}
function f(i){mk('gsAcc');gsAcc=(delete globalThis.gsAcc,i);return own('gsAcc');}
function g(i){'use strict';mk('gsRo');gsRo=(delete globalThis.gsRo,i);}
var r=[],errs=0;
for(var fn of [a,b,c,d,e]){try{r.push(fn.name+':'+fn());}catch(x){r.push(fn.name+':!'+x.name);}}
for(var i=0;i<=60;i++){r[5]=f(i);try{g(i);}catch(x){if(x instanceof TypeError)errs++;}}
r.push(errs,own('gsRo'),log.length,log[log.length-1]);
return r.join(' ');})()
