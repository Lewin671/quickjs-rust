(function(){var log=[],s=0,shared={[Symbol.dispose](){s++;}};
for(var i=0;i<1000;i++){for(using r=shared;;){break;}}
log.push(s);
for(var i=0;i<1000;i++){for(using r=shared;s<0;){}}
log.push(s);
for(var i=0;i<1000;i++){L:for(;;){try{break L;}finally{break L;}}}
for(var i=0;i<1000;i++){for(var x of [1]){try{break;}finally{continue;}}}
for(var i=0;i<1000;i++){for(var x of [1,2]){using q=shared;if(x>1)break;}}
log.push(s);
function f(n){var c=0;for(var i=0;i<n;i++){with({}){c++;break;}}for(var j=0;j<n;j++){M:{with({}){{c++;break M;}}}}return c;}
log.push(f(1000));
function g(){var t=0;for(var k=0;k<1000;k++){{using a=shared;}t++;}return t+s;}
log.push(g());
return log.join(',');})()
