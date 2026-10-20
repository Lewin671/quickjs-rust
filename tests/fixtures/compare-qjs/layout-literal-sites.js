(function(){
var out=[];
function keys(o){return Object.getOwnPropertyNames(o).join(",");}
function mk(i){ return {a:i,b:2,c:3}; }
function mk2(i){ return {x:i,y:2}; }
var l1=mk(1), l2=mk(2); Object.freeze(l1); delete l2.b; var l3=mk(3);
out.push(Object.isFrozen(l1), Object.isFrozen(l3), keys(l2), keys(l3));
var p1=mk2(1), p2=mk2(2); Object.defineProperty(p1,"x",{enumerable:false}); var p3=mk2(3);
out.push(Object.keys(p1).join(","), Object.keys(p3).join(","), keys(p2));
var d1={a:1,b:2,a:3}; var d2={a:1,b:2,a:3}; d2.c=4; out.push(keys(d1), d1.a, keys(d2));
for (var i=0;i<3;i++){ var q=mk(i); q.d=i; delete q.a; out.push(keys(q)); }
var sealed = mk(9); Object.seal(sealed); out.push(Object.isSealed(sealed), Object.isSealed(mk(9)));
var g = (0,eval)("({a:1,b:2,c:3})"); out.push(keys(g));
var arr=[]; for (var i=0;i<40;i++){ arr.push({k:i, v:i*2}); } arr[5].extra=1; out.push(keys(arr[5]), keys(arr[6]), arr[39].v);
return out.join(" | ");
})()
