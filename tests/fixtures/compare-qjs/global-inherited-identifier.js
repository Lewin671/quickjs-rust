(function(){Object.prototype.protoG='fromProto';var out=[protoG,typeof protoG];
var reads=0;Object.defineProperty(Object.prototype,'protoG',{configurable:true,get(){reads++;return this===globalThis?42:-1;}});
out.push(protoG,(function(){return protoG;})(),eval('protoG'),reads);function run(){function h(x){return protoG+x;}var s=0;for(var i=0;i<100;i++)s+=h(i);return s;}
out.push(run(),reads);return out.join(':');})()
