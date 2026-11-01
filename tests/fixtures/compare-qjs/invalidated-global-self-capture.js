(function(){globalThis.deletedCapture=1;var f=function nf(){nf;return deletedCapture;};
delete globalThis.deletedCapture;var missing;try{f();}catch(e){missing=e.name;}
Object.defineProperty(globalThis,'deletedCapture',{configurable:true,get(){return 7;}});
var accessor=f();delete globalThis.deletedCapture;globalThis.deletedCapture=9;
return missing+':'+accessor+':'+f();})()
