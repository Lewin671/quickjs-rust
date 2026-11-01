(function(){function top(obj){return obj.child();}var obj={child(){return this.bad.key;}};
var out=[];for(var i=0;i<3;i++){try{top(obj);}catch(e){out.push(e.name);}out.push((function(){return this.v;}).call({v:7}));}
return out.join(':');})()
