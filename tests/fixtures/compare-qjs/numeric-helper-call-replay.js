(function(){var out=[];for(var mode=0;mode<3;mode++){
var result=(function(){var n=0;function h(x){n++;return x;}var s=0;
for(var i=0;i<100;i++){s+=h(i);if(mode===1&&i===2){s+={valueOf(){return 0;}};}if(mode===2){h(i);}}
return n+':'+s;})();out.push(result);}return out.join('|');})()
