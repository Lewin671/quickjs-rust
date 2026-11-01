const run=Function("deletedCapture=undefined;var clean=()=>eval('delete deletedCapture');var p=(async function(){await 0;return deletedCapture=readyCapture?deletedCapture:1;})().then(v=>'ok:'+v,e=>e.name);readyCapture=1;clean();return p;");
print(await run());
