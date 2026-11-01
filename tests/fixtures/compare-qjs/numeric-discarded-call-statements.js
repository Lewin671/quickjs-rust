(function () {
var out=[];
out.push((0,eval)('var numericEffectCount=0;function numericSide(){numericEffectCount++;}function numericDiscard(a,b){numericSide();return a-b;}numericDiscard(2,1);numericEffectCount;'));
var n=0;
function side(){n++;}
function h(a,b){side();return a-b;}
h(2,1);out.push(n);n=0;
h.call(null,2,1);out.push(n);n=0;
h.apply(null,[2,1]);out.push(n);n=0;
Reflect.apply(h,null,[2,1]);out.push(n);n=0;
h.bind(null)(2,1);out.push(n);n=0;
[2,1].sort(h);out.push(n>0);n=0;
[2,1].toSorted(h);out.push(n>0);n=0;
new Uint8Array([2,1]).sort(h);out.push(n>0);n=0;
new Uint8Array([2,1]).toSorted(h);out.push(n>0);n=0;
Array.from([2,1],h);out.push(n);
var marker={};
function fail(){throw marker;}
function comparator(a,b){fail();return a-b;}
try{[2,1].sort(comparator);out.push(false);}catch(e){out.push(e===marker);}
var touches=0, obj={get value(){touches++;return 2;},valueOf(){touches++;return 2;}};
function reads(a,b){obj.value;+obj;return a-b;}
reads(2,1);out.push(touches);
return out.join(':');
})()
