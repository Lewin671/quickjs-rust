(function () {
function p() { var r=0; for(var i=0;i<3;i++){if(this.w==0) r+=this.v;} return r; }
var o={w:0,v:2,p:p};
function calc() { return o.p(); }
function P() { this.w=5; for(var i=0;i<3;i++){if(this.w!=1) this.w=0;} }
function make() { return new P().w; }
class C { constructor(){this.w=0;this.v=2;} m(){var r=0;var i=0;do{r+=this.w===0?this.v:0;}while(++i<3);return r;} }
var method={a:[0],m(k){this.a['x'+k]=1;return this.a.length;}};
function forceMethodExit(){return method.m(1);}
function F(k){var a=[];a['x'+k]=1;this.q=6;}
function forceConstructorExit(){return new F('z').q;}
return [calc(),make(),[0].map(function(){return new C().m();})[0],
  p.call(o),p.apply(o,[]),Reflect.apply(p,o,[]),p.bind(o)(),[0].map(p,o)[0],Array.from([0],p,o)[0],
  forceMethodExit(),forceConstructorExit()].join(':');
})()
