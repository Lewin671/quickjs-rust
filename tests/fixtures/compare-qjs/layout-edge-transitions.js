(function(){
var out=[];
function keys(o){return Object.getOwnPropertyNames(o).join(",");}
var seen=[];
class A { a=1; b=(delete this.a, seen.push(keys(this)), 2); c=(seen.push(keys(this)),3); }
var x=new A(), y=new A(); out.push(keys(x), keys(y), seen.join("/"));
class B { constructor(){ return {z:1}; } }
class C extends B { f=5; g=6; }
var c1=new C(), c2=new C(); out.push(keys(c1), keys(c2), c1.f+c2.g);
var o1={}; o1.p=1; Object.defineProperty(o1,"q",{value:2,enumerable:false});
var o2={}; o2.p=1; o2.q=2; out.push(Object.keys(o1).join(","), Object.keys(o2).join(","), keys(o1), keys(o2));
var d={a:1,b:2,c:3}; delete d.b; d.b=4; out.push(keys(d), JSON.stringify(d));
var e={}; e.a=1; e.b=2; delete e.a; e.a=3; var f={}; f.a=1; f.b=2; out.push(keys(e), keys(f));
function P(){this.x=1;this.y=2;} var ps=[]; for(var i=0;i<50;i++)ps.push(new P()); ps[3].z=9; delete ps[4].x; out.push(keys(ps[3]),keys(ps[4]),keys(ps[5]));
var big={}; for(var i=0;i<40;i++)big["k"+i]=i; delete big.k5; big.k5=5; out.push(Object.keys(big).slice(0,6).join(","), Object.keys(big).length, big.k39);
var fr={a:1,b:2}; Object.freeze(fr); var fr2={a:1,b:2}; out.push(Object.isFrozen(fr), Object.isFrozen(fr2), (function(){"use strict"; try{fr.a=2;return "no";}catch(e){return e.name;}})());
var j=JSON.parse('{"b":1,"a":2,"b":3,"1":4}'); out.push(keys(j), j.b);
function g(){} g.extra=1; g.prototype.m=2; out.push(keys(g), Object.keys(g).join(","));
var arr=[1,2]; arr.foo=1; arr.bar=2; delete arr.foo; arr.foo=3; out.push(keys(arr));
var gs=[]; for (var i=0;i<3;i++){ var q={}; Object.defineProperty(q,"v",{get:function(){return 7;},configurable:true}); q.w=1; gs.push(keys(q)+":"+q.v); } out.push(gs.join(";"));
var proto={inh:1}; var ch=Object.create(proto); ch.own=2; var kk=["inh","own","none"]; out.push(kk.map(function(k){return ch[k];}).join(","));
var m={}; for (var i=0;i<200;i++){ var k="k"+(i%7); if (k in m) delete m[k]; else m[k]=i; } out.push(keys(m));
return out.join(" | ");
})()
