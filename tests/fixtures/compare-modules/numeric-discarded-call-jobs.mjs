const fulfilled = Function("var n=0;function side(){n++;}return Promise.resolve(3).then(function(a){side();return a-1;}).then(function(v){return n+':'+v;});");
const rejected = Function("var marker={};function side(){throw marker;}return Promise.resolve(3).then(function(a){side();return a-1;}).then(function(){return false;},function(e){return e===marker;});");
print(await fulfilled());
print(await rejected());
