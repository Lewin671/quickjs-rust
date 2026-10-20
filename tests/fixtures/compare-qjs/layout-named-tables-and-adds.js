(function(){
var out=[];
function keys(o){return Object.getOwnPropertyNames(o).join(",");}
function F(){} F.b=1; F.a=2; out.push(keys(F));
var g=function(){}; g.x=1; delete g.name; Object.defineProperty(g,"name",{value:"n",configurable:true}); out.push(keys(g));
var h=function(){}; Object.defineProperty(h,"length",{value:5}); h.z=1; out.push(keys(h), h.length);
var k=function(){}; k.q=1; k.prototype; k.r=2; delete k.q; k.q=3; out.push(keys(k));
class S { static b(){} static a(){} } out.push(keys(S));
var arr=[1,2,3]; arr.b=1; arr.a=2; arr[5]=6; delete arr.b; arr.b=3; out.push(keys(arr));
var big=[]; big.length=4294967295; try { big.push(1); } catch(e) { out.push(e.name); } out.push(big[4294967295], keys(big).slice(-1)[0]);
function P(v){ this.a=v; this.b=v; } new P(1); new P(2);
var seen=[]; Object.defineProperty(P.prototype,"b",{set:function(v){seen.push(v);},configurable:true});
var p=new P(3); out.push(keys(p), seen.join(","));
delete P.prototype.b; out.push(keys(new P(4)));
function Q(lock){ if (lock) Object.preventExtensions(this); this.a=1; } new Q(false); new Q(false);
out.push(keys(new Q(true)), keys(new Q(false)));
var ro=Object.create(Object.defineProperty({}, "x", {value:1, writable:false}));
function R(){ this.x=2; } R.prototype=ro; new R(); out.push(keys(new R()), new R().x);
var trap=[]; var px=new Proxy({}, {set:function(t,k,v,r){trap.push(k); return Reflect.set(t,k,v,r);}, defineProperty:function(t,k,d){trap.push("def:"+k); return Reflect.defineProperty(t,k,d);}});
function T(){ this.m=1; } T.prototype=px; new T(); new T(); out.push(trap.join(","));
class U { a=1; b=2; c=3; } new U(); Object.defineProperty(U.prototype,"b",{set:function(){throw 1;}}); var u=new U(); out.push(keys(u)+":"+u.b);
var frozen=Object.freeze({a:1,b:2}); var copy={a:1,b:2}; copy.a=5; out.push(frozen.a, copy.a, Object.isFrozen(frozen));
return out.join(" | ");
})()
