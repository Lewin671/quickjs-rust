(function(){var out=[];outer:for(var i=0;i<2;i++){{try{out.push(x);}catch(e){out.push(e.name);}let x=1;continue outer;}}
var f=[];outer2:for(var j=0;j<3;j++){{let x=j;f.push(()=>x);continue outer2;}}
return out.join(':')+'|'+f.map(g=>g()).join();})()
