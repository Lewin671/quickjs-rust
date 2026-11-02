(function () {
  var log=[];
  function* f(){for(var i=0;i<2;i++){try{try{continue;}finally{yield 'a'+i;try{yield 'x';}finally{yield 'y';}}}finally{yield 'b'+i;}}return 7;}
  var g=f(),r;do{r=g.next();log.push(r.value+':'+r.done);}while(!r.done);
  function* h(){try{try{yield 0;}finally{yield 1;try{yield 2;}finally{yield 3;}}}finally{yield 4;}}
  var q=h();log.push(q.next().value);log.push(q.return(9).value);do{r=q.next();log.push(r.value+':'+r.done);}while(!r.done);
  return log.join(',');
})()
